#include "engine.h"

#include <QDir>
#include <QDirIterator>
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
    const QFileInfo info(path);
    const qint64 mtime = info.lastModified().toSecsSinceEpoch();
    const qint64 size = info.size();

    const auto cached = m_sniffed.constFind(path);
    if (cached != m_sniffed.cend() && cached->mtime == mtime && cached->size == size)
        return cached->icon;

    QMimeDatabase db;
    const QString name = db.mimeTypeForFile(path, QMimeDatabase::MatchDefault).iconName();
    const QString icon = name.isEmpty() ? QStringLiteral("text-x-generic") : name;
    m_sniffed.insert(path, { icon, mtime, size });
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
    // A path written with a trailing separator names the same folder and has no step on the end.
    while (walk.size() > 1 && walk.endsWith(QLatin1Char('/')))
        walk.chop(1);
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

QString Engine::kindOf(const QString &path) const {
    const QFileInfo info(path);
    if (info.isDir())
        return QStringLiteral("Folder");
    QMimeDatabase db;
    return db.mimeTypeForFile(info, QMimeDatabase::MatchDefault).comment();
}

QVariantList Engine::infoFor(const QString &path) const {
    const QFileInfo info(path);
    QVariantList out;
    if (!info.exists())
        return out;

    const auto row = [&out](const QString &label, const QString &value) {
        if (!value.isEmpty())
            out.append(QVariantMap { { QStringLiteral("label"), label }, { QStringLiteral("value"), value } });
    };

    row(QStringLiteral("Kind"), kindOf(path));
    // A folder's size would mean walking all of it, which Get Info does only when asked.
    row(QStringLiteral("Size"), info.isDir() ? QString() : formatSize(info.size()));
    row(QStringLiteral("Where"), info.absolutePath());
    row(QStringLiteral("Created"), info.birthTime().isValid()
        ? QLocale().toString(info.birthTime(), QLocale::ShortFormat) : QString());
    row(QStringLiteral("Modified"), QLocale().toString(info.lastModified(), QLocale::ShortFormat));
    row(QStringLiteral("Owner"), info.owner());
    row(QStringLiteral("Group"), info.group());

    // The permissions as a shell would write them, since that is what they are.
    const QFile::Permissions p = info.permissions();
    QString mode;
    const QFile::Permission bits[] = { QFile::ReadOwner, QFile::WriteOwner, QFile::ExeOwner,
                                       QFile::ReadGroup, QFile::WriteGroup, QFile::ExeGroup,
                                       QFile::ReadOther, QFile::WriteOther, QFile::ExeOther };
    const char letters[] = "rwxrwxrwx";
    for (int i = 0; i < 9; ++i)
        mode += (p & bits[i]) ? QChar::fromLatin1(letters[i]) : QLatin1Char('-');
    row(QStringLiteral("Permissions"), mode);
    if (info.isSymLink())
        row(QStringLiteral("Points at"), info.symLinkTarget());
    return out;
}

QString Engine::sizeOfFolder(const QString &path) const { return sizeOf({ path }); }

QString Engine::sizeOf(const QStringList &paths) const {
    if (paths.isEmpty())
        return {};
    qint64 total = 0;
    for (const QString &path : paths) {
        const QFileInfo info(path);
        if (!info.isDir()) { total += info.size(); continue; }
        QDirIterator it(path, QDir::Files | QDir::Hidden | QDir::System | QDir::NoDotAndDotDot,
                        QDirIterator::Subdirectories);
        while (it.hasNext()) { it.next(); total += it.fileInfo().size(); }
    }
    return formatSize(total);
}

QString Engine::freeSpace(const QString &path) const {
    const QStorageInfo where(path);
    if (!where.isValid() || !where.isReady())
        return {};
    return formatSize(where.bytesAvailable()) + QStringLiteral(" free");
}

QString Engine::spaceOn(const QString &path) const {
    const QStorageInfo where(path);
    if (!where.isValid() || !where.isReady() || where.bytesTotal() <= 0)
        return {};
    return formatSize(where.bytesAvailable()) + QStringLiteral(" free of ") + formatSize(where.bytesTotal());
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
