#include "directory.h"

#include <QFile>
#include <QFileInfo>
#include <QMimeDatabase>
#include <QtConcurrent/QtConcurrentRun>

#include <dirent.h>
#include <fcntl.h>
#include <sys/stat.h>

namespace {

// readdir with one fstatat each, rather than QDirIterator: d_type answers the only question asked
// of most rows, and a stat that fails leaves an entry that is still worth listing.
QVector<DirEntry> scan(const QString &path) {
    QVector<DirEntry> out;
    const QByteArray native = QFile::encodeName(path);

    DIR *dir = opendir(native.constData());
    if (!dir)
        return out;

    const int fd = dirfd(dir);
    QMimeDatabase mime;
    // A directory holds far fewer suffixes than files, and a mime lookup costs microseconds each,
    // so the answer is kept per suffix. The empty suffix is a key like any other.
    QHash<QString, QString> iconForSuffix;
    out.reserve(256);

    while (struct dirent *ent = readdir(dir)) {
        const char *raw = ent->d_name;
        if (raw[0] == '.' && (raw[1] == '\0' || (raw[1] == '.' && raw[2] == '\0')))
            continue;

        DirEntry e;
        e.name = QFile::decodeName(raw);
        e.isHidden = raw[0] == '.';

        struct statx sb;
        const bool statted = statx(fd, raw, AT_SYMLINK_NOFOLLOW, STATX_TYPE | STATX_SIZE | STATX_MTIME, &sb) == 0;
        if (statted) {
            e.isSymlink = S_ISLNK(sb.stx_mode);
            e.isDir = S_ISDIR(sb.stx_mode);
            e.size = qint64(sb.stx_size);
            e.modified = QDateTime::fromSecsSinceEpoch(qint64(sb.stx_mtime.tv_sec));
        } else if (ent->d_type != DT_UNKNOWN) {
            e.isSymlink = ent->d_type == DT_LNK;
            e.isDir = ent->d_type == DT_DIR;
        }

        // A symlink is listed as what it points at, so a link to a folder opens like one.
        if (e.isSymlink) {
            struct statx target;
            if (statx(fd, raw, 0, STATX_TYPE, &target) == 0)
                e.isDir = S_ISDIR(target.stx_mode);
        }

        if (e.isDir) {
            e.iconName = QStringLiteral("folder");
        } else {
            const int dot = e.name.indexOf(QLatin1Char('.'), 1);
            const QString suffix = dot < 0 ? QString() : e.name.mid(dot + 1).toLower();
            auto cached = iconForSuffix.constFind(suffix);
            if (cached == iconForSuffix.cend()) {
                const QString icon = mime.mimeTypeForFile(e.name, QMimeDatabase::MatchExtension).iconName();
                cached = iconForSuffix.insert(suffix, icon.isEmpty() ? QStringLiteral("text-x-generic") : icon);
            }
            e.iconName = *cached;
        }
        out.append(e);
    }

    closedir(dir);
    return out;
}

} // namespace

Directory::Directory(QObject *parent) : QAbstractListModel(parent) {
    m_collator.setNumericMode(true);
    m_collator.setCaseSensitivity(Qt::CaseInsensitive);

    // A write lands as several events; the settle timer turns a copy into one rescan.
    m_settle.setSingleShot(true);
    m_settle.setInterval(150);
    connect(&m_settle, &QTimer::timeout, this, &Directory::startScan);
    connect(&m_fsWatcher, &QFileSystemWatcher::directoryChanged, this, [this] { m_settle.start(); });
    connect(&m_watcher, &QFutureWatcher<QVector<DirEntry>>::finished, this, &Directory::scanFinished);
}

Directory::~Directory() {
    m_watcher.disconnect();
    m_watcher.waitForFinished();
}

int Directory::rowCount(const QModelIndex &parent) const {
    return parent.isValid() ? 0 : int(m_rows.size());
}

QVariant Directory::data(const QModelIndex &index, int role) const {
    if (index.row() < 0 || index.row() >= m_rows.size())
        return {};

    const DirEntry &e = m_rows.at(index.row());
    switch (role) {
    case NameRole: return e.name;
    case PathRole: return m_path + QLatin1Char('/') + e.name;
    case IconNameRole: return e.iconName;
    case SizeRole: return e.size;
    case ModifiedRole: return e.modified;
    case IsDirRole: return e.isDir;
    case IsSymlinkRole: return e.isSymlink;
    case IsHiddenRole: return e.isHidden;
    default: return {};
    }
}

QHash<int, QByteArray> Directory::roleNames() const {
    return {
        { NameRole, "name" },
        { PathRole, "path" },
        { IconNameRole, "iconName" },
        { SizeRole, "size" },
        { ModifiedRole, "modified" },
        { IsDirRole, "isDir" },
        { IsSymlinkRole, "isSymlink" },
        { IsHiddenRole, "isHidden" },
    };
}

void Directory::setPath(const QString &path) {
    if (m_path == path)
        return;
    m_path = path;
    emit pathChanged();
    startScan();
}

void Directory::setShowHidden(bool on) {
    if (m_showHidden == on)
        return;
    m_showHidden = on;
    emit showHiddenChanged();
    rebuild();
}

void Directory::setSort(Sort sort) {
    if (m_sort == sort)
        return;
    m_sort = sort;
    emit sortChanged();
    rebuild();
}

void Directory::setSortOrder(Qt::SortOrder order) {
    if (m_sortOrder == order)
        return;
    m_sortOrder = order;
    emit sortOrderChanged();
    rebuild();
}

void Directory::setFilter(const QString &filter) {
    if (m_filter == filter)
        return;
    m_filter = filter;
    emit filterChanged();
    rebuild();
}

void Directory::refresh() { startScan(); }

QString Directory::pathAt(int row) const {
    if (row < 0 || row >= m_rows.size())
        return {};
    return m_path + QLatin1Char('/') + m_rows.at(row).name;
}

bool Directory::isDirAt(int row) const {
    return row >= 0 && row < m_rows.size() && m_rows.at(row).isDir;
}

int Directory::rowOf(const QString &name) const {
    for (int i = 0; i < m_rows.size(); ++i)
        if (m_rows.at(i).name == name)
            return i;
    return -1;
}

void Directory::startScan() {
    m_settle.stop();

    if (!m_fsWatcher.directories().isEmpty())
        m_fsWatcher.removePaths(m_fsWatcher.directories());

    if (m_path.isEmpty()) {
        m_all.clear();
        rebuild();
        setStatus(Idle);
        return;
    }

    const QFileInfo info(m_path);
    if (!info.isDir()) {
        m_all.clear();
        rebuild();
        setStatus(Error, QStringLiteral("Not a folder"));
        return;
    }
    if (!info.isReadable()) {
        m_all.clear();
        rebuild();
        setStatus(Error, QStringLiteral("You do not have permission to open this folder"));
        return;
    }

    setStatus(Loading);
    m_watchedGeneration = ++m_generation;
    const QString path = m_path;
    m_watcher.setFuture(QtConcurrent::run([path] { return scan(path); }));
}

void Directory::scanFinished() {
    if (m_watchedGeneration != m_generation)
        return;

    m_all = m_watcher.result();
    rebuild();
    setStatus(Ready);

    if (!m_path.isEmpty())
        m_fsWatcher.addPath(m_path);
}

void Directory::rebuild() {
    // Collating a name is the expensive half of the sort, so each is turned into a key once here
    // rather than run through the collator on every one of n log n comparisons.
    struct Sortable {
        const DirEntry *entry;
        QCollatorSortKey name;
        QCollatorSortKey kind;
    };

    std::vector<Sortable> sortable;
    sortable.reserve(size_t(m_all.size()));
    for (const DirEntry &e : std::as_const(m_all)) {
        if (e.isHidden && !m_showHidden)
            continue;
        if (!m_filter.isEmpty() && !e.name.contains(m_filter, Qt::CaseInsensitive))
            continue;
        sortable.push_back({ &e,
                             m_collator.sortKey(e.name),
                             m_collator.sortKey(m_sort == ByKind ? e.iconName : QString()) });
    }

    // Folders lead whatever the sort and whichever direction it runs, as every file manager does.
    const int direction = m_sortOrder == Qt::AscendingOrder ? 1 : -1;
    const Sort by = m_sort;
    std::sort(sortable.begin(), sortable.end(), [direction, by](const Sortable &a, const Sortable &b) {
        if (a.entry->isDir != b.entry->isDir)
            return a.entry->isDir;
        int cmp = 0;
        switch (by) {
        case ByName: break;
        case BySize: cmp = a.entry->size < b.entry->size ? -1 : a.entry->size > b.entry->size ? 1 : 0; break;
        case ByModified: cmp = a.entry->modified < b.entry->modified ? -1 : a.entry->modified > b.entry->modified ? 1 : 0; break;
        case ByKind: cmp = a.kind.compare(b.kind); break;
        }
        if (cmp == 0)
            cmp = a.name.compare(b.name);
        return direction * cmp < 0;
    });

    QVector<DirEntry> rows;
    rows.reserve(int(sortable.size()));
    for (const Sortable &s : sortable)
        rows.append(*s.entry);

    beginResetModel();
    m_rows = std::move(rows);
    endResetModel();
    emit countChanged();
}

void Directory::setStatus(Status status, const QString &error) {
    if (m_status == status && m_error == error)
        return;
    m_status = status;
    m_error = error;
    emit statusChanged();
}
