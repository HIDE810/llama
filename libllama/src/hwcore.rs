use std::sync::{self, atomic};

use utils::task::{self, TaskMgmt};

use cpu;
use ldr;
use mem;
use io;

fn map_memory_regions(arm9_io: io::IoRegsArm9, shared_io: io::IoRegsShared)
        -> (mem::MemController, mem::MemController, mem::MemController) {
    let arm9_itcm = mem::MemoryBlock::make_ram(0x20);
    let arm9_ram = mem::MemoryBlock::make_ram(0x400);
    let arm9_io = mem::MemoryBlock::make_io(io::IoRegion::Arm9(arm9_io), 0x400);
    let arm9_dtcm = mem::MemoryBlock::make_ram(0x10);
    let arm9_bootrom = mem::MemoryBlock::make_ram(0x40);

    let shared_io = mem::MemoryBlock::make_io(io::IoRegion::Shared(shared_io), 0x400);
    let vram = mem::MemoryBlock::make_ram(0x1800);
    let dsp_ram = mem::MemoryBlock::make_ram(0x200);
    let axi_wram = mem::MemoryBlock::make_ram(0x200);
    let fcram = mem::MemoryBlock::make_ram(0x20000);

    let mut controller9 = mem::MemController::new();
    for i in 0..0x1000 {
        controller9.map_region(i * 0x8000, arm9_itcm.clone());
    }
    controller9.map_region(0x08000000, arm9_ram.clone());
    controller9.map_region(0x10000000, arm9_io.clone());
    controller9.map_region(0x10100000, shared_io.clone());
    controller9.map_region(0x18000000, vram.clone());
    controller9.map_region(0x1FF00000, dsp_ram.clone());
    controller9.map_region(0x1FF80000, axi_wram.clone());
    controller9.map_region(0x20000000, fcram.clone());
    controller9.map_region(0xFFF00000, arm9_dtcm.clone());
    controller9.map_region(0xFFFF0000, arm9_bootrom.clone());

    let mut controller11 = mem::MemController::new();
    controller11.map_region(0x1FF80000, axi_wram.clone());
    controller11.map_region(0x20000000, fcram.clone());

    let mut controller_pica = mem::MemController::new();
    controller_pica.map_region(0x20000000, fcram.clone());

    return (controller9, controller11, controller_pica);
}

pub struct Hardware9 {
    pub arm9: cpu::Cpu
}

pub struct Hardware11 {
    pub dummy11: cpu::dummy11::Dummy11
}

pub struct HwCore {
    hardware9: sync::Arc<sync::RwLock<Hardware9>>,
    hardware11: sync::Arc<sync::RwLock<Hardware11>>,

    hardware_task: Option<task::Task>,
    arm11_handshake_task: Option<task::Task>,

    mem_pica: mem::MemController,
}

impl HwCore {
    pub fn new(loader: &ldr::Loader) -> HwCore {
        let (mut mem9, mem11, mem_pica) = map_memory_regions(io::IoRegsArm9::new(), io::IoRegsShared::new());
        loader.load(&mut mem9);

        let mut cpu = cpu::Cpu::new(mem9);
        cpu.reset(loader.entrypoint());

        HwCore {
            hardware9: sync::Arc::new(sync::RwLock::new(Hardware9 {
                arm9: cpu
            })),
            hardware11: sync::Arc::new(sync::RwLock::new(Hardware11 {
                dummy11: cpu::dummy11::Dummy11::new(mem11, cpu::dummy11::modes::kernel())
            })),
            hardware_task: None,
            arm11_handshake_task: None,
            mem_pica: mem_pica,
        }
    }

    // Spin up the hardware thread, take ownership of hardware
    pub fn start(&mut self) {
        let hardware9 = self.hardware9.clone();
        let hardware11 = self.hardware11.clone();

        self.hardware_task = Some(task::Task::spawn(move |running| {
            // Nobody else can access the hardware while the thread runs
            let mut hardware = hardware9.write().unwrap();

            while running.load(atomic::Ordering::SeqCst) {
                if let cpu::BreakReason::Breakpoint = hardware.arm9.run(1000) {
                    info!("Breakpoint hit @ 0x{:X}!", hardware.arm9.regs[15] - hardware.arm9.get_pc_offset());
                    break;
                }
            }
        }));

        // On reset, the ARM9 and ARM11 processors perform a handshake, where
        // the two processors synchronize over AXI WRAM address 0x1FFFFFF0.
        // Until the ARM11 is emulated, manually doing this will allow FIRM to boot.
        self.arm11_handshake_task = Some(task::Task::spawn(move |running| {
            // Nobody else can access the hardware while the thread runs
            let mut hardware = hardware11.write().unwrap();

            use std::{thread, time};

            while running.load(atomic::Ordering::SeqCst) {
                if let cpu::BreakReason::Breakpoint = hardware.dummy11.step() {
                    thread::sleep(time::Duration::from_millis(10));
                }
            }
        }));
    }

    fn panic_action(&self) -> ! {
        // Join failed, uh oh
        if let Err(poisoned) = self.hardware9.read() {
            let hw = poisoned.into_inner();
            panic!("CPU thread panicked! PC: 0x{:X}, LR: 0x{:X}", hw.arm9.regs[15], hw.arm9.regs[14]);
        }
        panic!("CPU thread panicked!");
    }

    pub fn try_wait(&mut self) -> bool {
        let res = {
            let mut tasks = [
                (&mut self.hardware_task, task::EndBehavior::StopAll),
                (&mut self.arm11_handshake_task, task::EndBehavior::Ignore)
            ];
            task::TaskUnion(&mut tasks).try_join()
        };

        match res {
            Ok(x) => x == task::JoinStatus::Joined,
            Err(_) => self.panic_action()
        }
    }

    pub fn stop(&mut self) {
        let res = {
            let mut tasks = [
                (&mut self.hardware_task, task::EndBehavior::StopAll),
                (&mut self.arm11_handshake_task, task::EndBehavior::Ignore)
            ];

            task::TaskUnion(&mut tasks).stop()
        };

        if res.is_err() {
            self.panic_action()
        }
        self.hardware_task = None;
        self.arm11_handshake_task = None;
    }

    pub fn hardware(&self) -> sync::RwLockReadGuard<Hardware9> {
        // Will panic if already running
        self.hardware9.try_read().unwrap()
    }

    pub fn hardware_mut(&mut self) -> sync::RwLockWriteGuard<Hardware9> {
        // Will panic if already running
        self.hardware9.try_write().unwrap()
    }

    pub fn copy_framebuffers(&mut self, fbs: &mut Framebuffers) {
        fbs.top_screen.resize({ let (w, h, d) = fbs.top_screen_size; w*h*d }, 0);
        fbs.bot_screen.resize({ let (w, h, d) = fbs.bot_screen_size; w*h*d }, 0);

        self.mem_pica.read_buf(0x20000000u32, fbs.top_screen.as_mut_slice());
        // self.mem_pica.read_buf(0x20046500u32, ..);
        self.mem_pica.read_buf(0x2008CA00u32, fbs.bot_screen.as_mut_slice());
        // self.mem_pica.read_buf(0x200C4E00u32, ..);
    }
}

pub struct Framebuffers {
    pub top_screen: Vec<u8>,
    pub bot_screen: Vec<u8>,
    pub top_screen_size: (usize, usize, usize),
    pub bot_screen_size: (usize, usize, usize),
}