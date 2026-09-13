#pragma once

#include <QDateTime>
#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QVariantList>

// The browsing engine's own identity, and the mime lookup every view needs.
class Engine : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON
    Q_PROPERTY(QString version READ version CONSTANT)
    Q_PROPERTY(QString home READ home CONSTANT)

public:
    explicit Engine(QObject *parent = nullptr);

    QString version() const;
    QString home() const;

    // The icon theme name shared-mime-info gives this path, to hand to Quickshell.iconPath.
    // The name alone decides it, so a listing costs no reads; a file with no extension is
    // application-octet-stream whatever is inside it.
    Q_INVOKABLE QString iconNameFor(const QString &path) const;

    // Path arithmetic, so no view has to do string surgery. parentOf the root is the root.
    Q_INVOKABLE QString parentOf(const QString &path) const;
    Q_INVOKABLE QString join(const QString &dir, const QString &name) const;
    Q_INVOKABLE bool isDir(const QString &path) const;
    // The name to show for a path: the home directory is Home, the root is its separator.
    Q_INVOKABLE QString displayName(const QString &path) const;
    // The breadcrumb trail as [{ name, path }], from the root or from home when the path is under it.
    Q_INVOKABLE QVariantList crumbs(const QString &path) const;

    Q_INVOKABLE QString formatSize(qint64 bytes) const;
    Q_INVOKABLE QString formatModified(const QDateTime &when) const;
};
