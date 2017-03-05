#[macro_use]
extern crate log;
extern crate capstone;
extern crate lgl;
extern crate libc;
extern crate libllama;

mod commands;
mod uilog;

use std::env;

use libllama::{dbgcore, hwcore, ldr};

pub struct Backend {
    debugger: dbgcore::DbgCore,
    fbs: hwcore::Framebuffers
}

#[repr(C)]
pub struct FrontendCallbacks {
    set_running: extern fn(*mut Backend, bool),
    is_running: extern fn(*mut Backend) -> bool,
    top_screen: extern fn(*mut Backend, *mut usize) -> *const u8,
    bot_screen: extern fn(*mut Backend, *mut usize) -> *const u8,
    run_command: extern fn(*mut Backend, *const u8, usize),
}

extern {
    fn llama_open_gui(backend: *mut Backend, callbacks: *const FrontendCallbacks);
}


mod cbs {
    use std::slice;
    use std::str;

    use commands;
    use Backend;

    pub extern fn set_running(backend: *mut Backend, state: bool) {
        if state {
            unsafe { (*backend).debugger.ctx().resume(); }
        } else {
            unsafe { (*backend).debugger.ctx().pause(); }
        }
    }

    pub extern fn is_running(backend: *mut Backend) -> bool {
        unsafe { !(*backend).debugger.ctx().hwcore_mut().try_wait() }
    }

    pub extern fn top_screen(backend: *mut Backend, buf_size_out: *mut usize) -> *const u8 {
        let backend = unsafe { &mut *backend };
        backend.debugger.ctx().hwcore_mut().copy_framebuffers(&mut backend.fbs);
        unsafe {
            *buf_size_out = backend.fbs.top_screen.len();
            backend.fbs.top_screen.as_ptr()
        }
    }

    pub extern fn bot_screen(backend: *mut Backend, buf_size_out: *mut usize) -> *const u8 {
        let backend = unsafe { &mut *backend };
        backend.debugger.ctx().hwcore_mut().copy_framebuffers(&mut backend.fbs);
        unsafe {
            *buf_size_out = backend.fbs.bot_screen.len();
            backend.fbs.bot_screen.as_ptr()
        }
    }

    pub extern fn run_command(backend: *mut Backend, str_buf: *const u8, str_len: usize) {
        let backend = unsafe { &mut *backend };
        let input = unsafe {
            let slice = slice::from_raw_parts(str_buf, str_len);
            str::from_utf8(slice).unwrap()
        };

        for cmd in input.split(';') {
            use lgl;
            lgl::log("> ");
            lgl::log(cmd);
            lgl::log("\n");
            commands::handle(&mut backend.debugger, cmd.split_whitespace());
        }
    }
}

fn main() {
    uilog::init().unwrap();

    let path = env::args().nth(1).unwrap();
    let loader = ldr::Ctr9Loader::from_folder(&path).unwrap();

    let callbacks = FrontendCallbacks {
        set_running: cbs::set_running,
        is_running: cbs::is_running,
        top_screen: cbs::top_screen,
        bot_screen: cbs::bot_screen,
        run_command: cbs::run_command,
    };

    let fbs = hwcore::Framebuffers {
        top_screen: Vec::new(), bot_screen: Vec::new(),
        top_screen_size: (240, 400, 3), bot_screen_size: (240, 320, 3),
    };

    let hwcore = hwcore::HwCore::new(loader);
    let mut backend = Backend {
        debugger: dbgcore::DbgCore::bind(hwcore),
        fbs: fbs
    };

    unsafe { llama_open_gui(&mut backend, &callbacks) };
}