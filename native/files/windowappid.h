#pragma once

#include <QObject>
#include <QPointer>
#include <QQmlEngine>
#include <QString>
#include <QWindow>

// Gives one window of the shell its own Wayland app id (D92).
class WindowAppId : public QObject {
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(QWindow *window READ window WRITE setWindow NOTIFY windowChanged)
    Q_PROPERTY(QString appId READ appId WRITE setAppId NOTIFY appIdChanged)

public:
    explicit WindowAppId(QObject *parent = nullptr);

    QWindow *window() const;
    void setWindow(QWindow *window);
    QString appId() const;
    // Takes effect the next time the window is shown: a mapped window keeps the id it was mapped with.
    void setAppId(const QString &appId);

signals:
    void windowChanged();
    void appIdChanged();

private:
    void showing(bool visible);

    QPointer<QWindow> m_window;
    QMetaObject::Connection m_shown;
    QString m_appId;
};
