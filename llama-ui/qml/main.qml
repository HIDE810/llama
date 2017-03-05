import QtQuick 2.6
import QtQuick.Controls 2.1
import QtQuick.Layouts 1.3

Item {
    width: 880
    height: 480

    property ScreenView scrnView: scrnView
    property DbgConsole dbgConsole: dbgConsole

    RowLayout {
        anchors.fill: parent
        anchors.margins: 4

        ScreenView {
            id: scrnView
            Layout.fillWidth: true
            Layout.fillHeight: true
        }

        DbgConsole {
            id: dbgConsole
            Layout.preferredWidth: 400
            Layout.fillHeight: true
        }
    }
}