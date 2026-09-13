#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>

// The browsing engine's own identity, and the mime lookup every view needs.
class Engine : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON
    Q_PROPERTY(QString version READ version CONSTANT)

public:
    explicit Engine(QObject *parent = nullptr);

    QString version() const;

    // The icon theme name shared-mime-info gives this path, to hand to Quickshell.iconPath.
    // The name alone decides it, so a listing costs no reads; a file with no extension is
    // application-octet-stream whatever is inside it.
    Q_INVOKABLE QString iconNameFor(const QString &path) const;
};
