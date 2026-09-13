#include "engine.h"

#include <QDir>
#include <QFileInfo>
#include <QLocale>
#include <QStorageInfo>
#include <QStandardPaths>
#include <QVariantList>
#include <QMimeDatabase>
#include <QVariantMap>

Engine::Engine(QObject *parent) : QObject(parent) {}

QString Engine::version() const { return QStringLiteral(ISLE_FILES_VERSION); }

QString Engine::home() const { return QDir::homePath(); }

QString Engine::iconNameFor(const QString &path) const {
    QFileInfo info(path);
    if (info.isDir())
        return QStringLiteral("folder");

    QMimeDatabase db;
    const QMimeType type = db.mimeTypeForFile(info, QMimeDatabase::MatchExtension);
    const QString name = type.iconName();
    return name.isEmpty() ? QStringLiteral("text-x-generic") : name;
}

bool Engine::isGenericIcon(const QString &iconName) const {
    return iconName == QLatin1String("application-octet-stream") || iconName == QLatin1String("text-x-generic");
}

QString Engine::sniffIconName(const QString &path) const {
    const auto cached = m_sniffed.constFind(path);
    if (cached != m_sniffed.cend())
        return *cached;

    QMimeDatabase db;
    const QString name = db.mimeTypeForFile(path, QMimeDatabase::MatchDefault).iconName();
    const QString icon = name.isEmpty() ? QStringLiteral("text-x-generic") : name;
    m_sniffed.insert(path, icon);
    return icon;
}

QString Engine::parentOf(const QString &path) const {
    if (path.isEmpty() || path == QLatin1String("/"))
        return QStringLiteral("/");
    const QString parent = QFileInfo(path).absolutePath();
    return parent.isEmpty() ? QStringLiteral("/") : parent;
}

QString Engine::join(const QString &dir, const QString &name) const {
    if (dir.endsWith(QLatin1Char('/')))
        return dir + name;
    return dir + QLatin1Char('/') + name;
}

bool Engine::isDir(const QString &path) const { return QFileInfo(path).isDir(); }

QString Engine::displayName(const QString &path) const {
    if (path == QLatin1String("/"))
        return QStringLiteral("/");
    if (path == QDir::homePath())
        return QStringLiteral("Home");
    const QString name = QFileInfo(path).fileName();
    return name.isEmpty() ? path : name;
}

QVariantList Engine::crumbs(const QString &path) const {
    QVariantList out;
    if (path.isEmpty())
        return out;

    const QString home = QDir::homePath();
    // A path under home starts at Home rather than at the root, which is where a person thinks it starts.
    const bool underHome = path == home || path.startsWith(home + QLatin1Char('/'));
    const QString base = underHome ? home : QStringLiteral("/");

    QStringList names;
    QString walk = path;
    while (walk.size() > base.size()) {
        names.prepend(QFileInfo(walk).fileName());
        walk = parentOf(walk);
    }

    QString built = base;
    out.append(QVariantMap { { QStringLiteral("name"), displayName(base) }, { QStringLiteral("path"), base } });
    for (const QString &name : std::as_const(names)) {
        built = join(built, name);
        out.append(QVariantMap { { QStringLiteral("name"), name }, { QStringLiteral("path"), built } });
    }
    return out;
}

QString Engine::freeSpace(const QString &path) const {
    const QStorageInfo where(path);
    if (!where.isValid() || !where.isReady())
        return {};
    return formatSize(where.bytesAvailable()) + QStringLiteral(" free");
}

QString Engine::formatSize(qint64 bytes) const {
    if (bytes < 0)
        return {};
    return QLocale().formattedDataSize(bytes, 1, QLocale::DataSizeTraditionalFormat);
}

QString Engine::formatModified(const QDateTime &when) const {
    if (!when.isValid())
        return {};

    const QDateTime now = QDateTime::currentDateTime();
    // Today is a time, this year is a day and a month, anything older carries its year.
    if (when.date() == now.date())
        return QLocale().toString(when.time(), QLocale::ShortFormat);
    if (when.date().year() == now.date().year())
        return QLocale().toString(when.date(), QStringLiteral("d MMM"));
    return QLocale().toString(when.date(), QStringLiteral("d MMM yyyy"));
}
