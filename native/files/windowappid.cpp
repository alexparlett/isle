#include "windowappid.h"

#include <QGuiApplication>
#include <QTimer>

namespace {

// The process's own desktop file name while a window's stands in for it.
QString processName;
bool swapped = false;

} // namespace

WindowAppId::WindowAppId(QObject *parent) : QObject(parent) {}

QWindow *WindowAppId::window() const { return m_window; }

void WindowAppId::setWindow(QWindow *window) {
    if (m_window == window)
        return;
    disconnect(m_shown);
    m_window = window;
    // visibleChanged is emitted before the platform window maps, and the Wayland platform reads the
    // application's desktop file name as the app id while it maps.
    if (m_window)
        m_shown = connect(m_window, &QWindow::visibleChanged, this, &WindowAppId::showing, Qt::DirectConnection);
    emit windowChanged();
}

QString WindowAppId::appId() const { return m_appId; }

void WindowAppId::setAppId(const QString &appId) {
    if (m_appId == appId)
        return;
    m_appId = appId;
    emit appIdChanged();
}

void WindowAppId::showing(bool visible) {
    if (!visible || m_appId.isEmpty())
        return;
    // Put back once the show in progress has run, so nothing else the process names carries a window's id.
    if (!swapped) {
        processName = QGuiApplication::desktopFileName();
        swapped = true;
        QTimer::singleShot(0, qApp, [] {
            QGuiApplication::setDesktopFileName(processName);
            swapped = false;
        });
    }
    QGuiApplication::setDesktopFileName(m_appId);
}
