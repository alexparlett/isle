#pragma once

#include <QDateTime>
#include <QHash>
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
    // The same question answered by reading the file rather than trusting its name, for the handful
    // of rows on screen whose name said nothing. Answers are kept, since a view asks repeatedly.
    Q_INVOKABLE QString sniffIconName(const QString &path) const;
    // What iconNameFor gives a file whose name says nothing, so a view knows when to ask again.
    Q_INVOKABLE bool isGenericIcon(const QString &iconName) const;

    // Path arithmetic, so no view has to do string surgery. parentOf the root is the root.
    Q_INVOKABLE QString parentOf(const QString &path) const;
    Q_INVOKABLE QString join(const QString &dir, const QString &name) const;
    Q_INVOKABLE bool isDir(const QString &path) const;
    // The name to show for a path: the home directory is Home, the root is its separator.
    Q_INVOKABLE QString displayName(const QString &path) const;
    // The breadcrumb trail as [{ name, path }], from the root or from home when the path is under it.
    Q_INVOKABLE QVariantList crumbs(const QString &path) const;

    // What is free on the volume a path is on, already said in words. Empty when it cannot be asked.
    Q_INVOKABLE QString freeSpace(const QString &path) const;
    // The same with what the volume holds altogether: "108.7 GB free of 1.8 TB".
    Q_INVOKABLE QString spaceOn(const QString &path) const;
    // How much these paths come to, folders counted through. Empty for nothing.
    Q_INVOKABLE QString sizeOf(const QStringList &paths) const;

    // What Get Info shows: [{ label, value }] for one path, read without walking into folders.
    Q_INVOKABLE QVariantList infoFor(const QString &path) const;
    // How much is inside a folder, counted through. Slow by nature, so it is asked for, not offered.
    Q_INVOKABLE QString sizeOfFolder(const QString &path) const;
    // The mime type's own description, as "PNG image" rather than "image/png".
    Q_INVOKABLE QString kindOf(const QString &path) const;

    Q_INVOKABLE QString formatSize(qint64 bytes) const;
    Q_INVOKABLE QString formatModified(const QDateTime &when) const;

private:
    // What reading a file said its icon was, against the state of the file when it was read.
    struct Sniffed {
        QString icon;
        qint64 mtime = 0;
        qint64 size = 0;
    };
    mutable QHash<QString, Sniffed> m_sniffed;
};
