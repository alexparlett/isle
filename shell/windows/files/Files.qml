import QtQuick
import Quickshell
import qs.services
// Loaded by URL rather than declared (see shell.qml), which leaves no implicit scope for a sibling.
import qs.windows.files

// One window per entry in FileWindows, so Files is as many windows as are wanted rather than one.
Scope {
    Variants {
        model: FileWindows.windows
        delegate: FilesWindow {}
    }
}
