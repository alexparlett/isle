#include "directory.h"

#include <QFile>
#include <QFileInfo>
#include <QLocale>
#include <QMimeDatabase>
#include <QtConcurrent/QtConcurrentRun>

#include <dirent.h>
#include <fcntl.h>
#include <sys/inotify.h>
#include <unistd.h>
#include <sys/stat.h>

namespace {

// The same row a scan would make, for a path named on its own rather than found in a folder.
DirEntry entryFor(const QString &path, QMimeDatabase &mime) {
    DirEntry e;
    e.path = path;
    const QFileInfo info(path);
    e.name = info.fileName();
    e.isHidden = e.name.startsWith(QLatin1Char('.'));
    e.isSymlink = info.isSymLink();
    e.isDir = info.isDir();
    e.size = info.size();
    e.modified = info.lastModified();
    e.iconName = e.isDir ? QStringLiteral("folder")
                         : mime.mimeTypeForFile(e.name, QMimeDatabase::MatchExtension).iconName();
    if (e.iconName.isEmpty())
        e.iconName = QStringLiteral("text-x-generic");
    return e;
}

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
    connect(&m_watcher, &QFutureWatcher<QVector<DirEntry>>::finished, this, &Directory::scanFinished);

    m_inotify = inotify_init1(IN_NONBLOCK | IN_CLOEXEC);
    if (m_inotify >= 0) {
        m_notifier = new QSocketNotifier(m_inotify, QSocketNotifier::Read, this);
        connect(m_notifier, &QSocketNotifier::activated, this, [this] {
            char buffer[4096];
            while (read(m_inotify, buffer, sizeof(buffer)) > 0) { }
            m_settle.start();
        });
    }
}

Directory::~Directory() {
    m_watcher.disconnect();
    m_watcher.waitForFinished();
    unwatch();
    if (m_inotify >= 0)
        close(m_inotify);
}

void Directory::watch(const QString &path) {
    unwatch();
    if (m_inotify < 0)
        return;
    // Everything that changes what a row says: an entry appearing or going, and a file being
    // written, renamed or having its permissions changed.
    m_watch = inotify_add_watch(m_inotify, QFile::encodeName(path).constData(),
                                IN_CREATE | IN_DELETE | IN_MOVED_FROM | IN_MOVED_TO | IN_MOVE_SELF |
                                IN_DELETE_SELF | IN_CLOSE_WRITE);
}

void Directory::unwatch() {
    if (m_inotify >= 0 && m_watch >= 0)
        inotify_rm_watch(m_inotify, m_watch);
    m_watch = -1;
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
    case PathRole: return pathOf(e);
    case IconNameRole: return e.iconName;
    case SizeRole: return e.size;
    case ModifiedRole: return e.modified;
    case IsDirRole: return e.isDir;
    case IsSymlinkRole: return e.isSymlink;
    case IsHiddenRole: return e.isHidden;
    case GroupRole: return groupOf(e);
    case DepthRole: return e.depth;
    case ExpandedRole: return m_expanded.contains(pathOf(e));
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
        { GroupRole, "groupName" },
        { DepthRole, "nestDepth" },
        { ExpandedRole, "opened" },
    };
}

void Directory::setPath(const QString &path) {
    if (m_path == path)
        return;
    m_path = path;
    m_expanded.clear();
    m_inside.clear();
    if (!m_paths.isEmpty()) {
        m_paths.clear();
        emit pathsChanged();
    }
    emit pathChanged();
    startScan();
}

void Directory::setPaths(const QStringList &paths) {
    if (m_paths == paths)
        return;
    m_paths = paths;
    emit pathsChanged();

    unwatch();
    m_settle.stop();
    // A scan of the folder may still be in flight; its rows would land on top of these. Moving the
    // generation on is what tells it nobody is waiting for it any more.
    ++m_generation;
    // A gathering is built here and now: the rows are named, so there is nothing to go and find.
    m_all.clear();
    QMimeDatabase mime;
    for (const QString &one : m_paths)
        if (QFileInfo::exists(one))
            m_all.append(entryFor(one, mime));
    m_scannedPath.clear();
    rebuild();
    setStatus(Ready);
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

void Directory::setPatterns(const QStringList &patterns) {
    if (m_patterns == patterns)
        return;
    m_patterns = patterns;
    // Compiled once here rather than per row per rebuild.
    m_globs.clear();
    for (const QString &glob : patterns)
        m_globs.append(QRegularExpression::fromWildcard(glob, Qt::CaseInsensitive,
                                                        QRegularExpression::UnanchoredWildcardConversion));
    emit patternsChanged();
    rebuild();
}

void Directory::setGrouping(Grouping grouping) {
    if (m_grouping == grouping)
        return;
    m_grouping = grouping;
    emit groupingChanged();
    // The rows do not change, only the heading each sits under, and rebuild() keeps the model as it
    // is when the rows match. The view is told the one role that did change.
    if (!m_rows.isEmpty())
        emit dataChanged(index(0, 0), index(m_rows.size() - 1, 0), { GroupRole });
}

void Directory::setFoldersOnly(bool on) {
    if (m_foldersOnly == on)
        return;
    m_foldersOnly = on;
    emit foldersOnlyChanged();
    rebuild();
}

void Directory::refresh() { startScan(); }

QString Directory::pathOf(const DirEntry &entry) const {
    if (!entry.path.isEmpty())
        return entry.path;
    const QString base = m_scannedPath.isEmpty() ? m_path : m_scannedPath;
    return base.endsWith(QLatin1Char('/')) ? base + entry.name : base + QLatin1Char('/') + entry.name;
}

QString Directory::pathAt(int row) const {
    if (row < 0 || row >= m_rows.size())
        return {};
    return pathOf(m_rows.at(row));
}

bool Directory::isDirAt(int row) const {
    return row >= 0 && row < m_rows.size() && m_rows.at(row).isDir;
}

void Directory::expand(int row) {
    const QString path = pathAt(row);
    if (path.isEmpty() || !isDirAt(row) || m_expanded.contains(path))
        return;
    m_expanded.insert(path);
    // Opened now, so what is inside it is read now rather than from whenever it was last looked at.
    m_inside.remove(path);
    rebuild();
}

void Directory::collapse(int row) {
    const QString path = pathAt(row);
    if (path.isEmpty() || !m_expanded.remove(path))
        return;
    // Anything opened inside it is shut too, so opening it again does not unfold the lot.
    for (auto it = m_expanded.begin(); it != m_expanded.end();)
        it = it->startsWith(path + QLatin1Char('/')) ? m_expanded.erase(it) : ++it;
    for (auto it = m_inside.begin(); it != m_inside.end();)
        it = (it.key() == path || it.key().startsWith(path + QLatin1Char('/'))) ? m_inside.erase(it) : ++it;
    rebuild();
}

QString Directory::groupAt(int row) const {
    if (row < 0 || row >= m_rows.size())
        return {};
    return groupOf(m_rows.at(row));
}

int Directory::startingWith(const QString &prefix, int from) const {
    if (prefix.isEmpty())
        return -1;
    for (int i = qMax(0, from); i < m_rows.size(); ++i)
        if (m_rows.at(i).name.startsWith(prefix, Qt::CaseInsensitive))
            return i;
    return -1;
}

int Directory::rowOf(const QString &name) const {
    for (int i = 0; i < m_rows.size(); ++i)
        if (m_rows.at(i).name == name)
            return i;
    return -1;
}

int Directory::rowOfPath(const QString &path) const {
    for (int i = 0; i < m_rows.size(); ++i)
        if (pathOf(m_rows.at(i)) == path)
            return i;
    return -1;
}

void Directory::startScan() {
    m_settle.stop();
    // A gathering is not a folder: there is nothing to go and read, and reading would throw its
    // rows away.
    if (!m_paths.isEmpty())
        return;

    unwatch();

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

    // Only a folder not yet shown is "loading". A rescan of the one on screen keeps its rows and its
    // status, so nothing flashes and nothing is rebuilt under the pointer.
    if (m_scannedPath != m_path)
        setStatus(Loading);
    m_watchedGeneration = ++m_generation;
    const QString path = m_path;
    m_watcher.setFuture(QtConcurrent::run([path] { return scan(path); }));
}

void Directory::scanFinished() {
    if (m_watchedGeneration != m_generation)
        return;

    m_all = m_watcher.result();
    m_scannedPath = m_path;
    m_inside.clear();
    rebuild();
    setStatus(Ready);

    if (!m_path.isEmpty())
        watch(m_path);
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
        if (!keeps(e))
            continue;
        sortable.push_back({ &e,
                             m_collator.sortKey(e.name),
                             m_collator.sortKey(m_sort == ByKind ? e.iconName : QString()) });
    }

    // A gathering keeps the order it was given: what was opened lately is in the order it was
    // opened, and sorting that by name would throw away what makes it useful.
    if (!m_paths.isEmpty()) {
        QVector<DirEntry> kept;
        kept.reserve(int(sortable.size()));
        for (const Sortable &one : sortable)
            kept.append(*one.entry);
        beginResetModel();
        m_rows = std::move(kept);
        m_rowsBase.clear();
        m_rowsExpanded.clear();
        endResetModel();
        emit countChanged();
        return;
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
    for (const Sortable &s : sortable) {
        rows.append(*s.entry);
        // A folder opened in place brings what is inside it along, under the row that opened it.
        const QString path = pathOf(*s.entry);
        if (s.entry->isDir && m_expanded.contains(path)) {
            rows.last().path = path;
            appendExpanded(rows, path, 1);
        }
    }

    // Rebuilding a listing that has not changed would throw away where the view is and what is
    // picked. Rows that read the same but came from a different folder are a different listing.
    const QString base = m_scannedPath.isEmpty() ? m_path : m_scannedPath;
    if (base == m_rowsBase && m_expanded == m_rowsExpanded && rows.size() == m_rows.size()) {
        bool same = true;
        for (int i = 0; i < rows.size() && same; ++i) {
            const DirEntry &a = rows.at(i), &b = m_rows.at(i);
            same = a.name == b.name && a.size == b.size && a.modified == b.modified
                && a.isDir == b.isDir && a.depth == b.depth && a.path == b.path
                && a.iconName == b.iconName && a.isSymlink == b.isSymlink && a.isHidden == b.isHidden;
        }
        if (same)
            return;
    }

    beginResetModel();
    m_rows = std::move(rows);
    m_rowsBase = base;
    m_rowsExpanded = m_expanded;
    endResetModel();
    emit countChanged();
}

// The heading a row belongs under, in the words Finder uses.
QString Directory::groupOf(const DirEntry &e) const {
    switch (m_grouping) {
    case NoGroups:
        return {};
    case ByKindGroups:
        return e.isDir ? QStringLiteral("Folders") : QMimeDatabase()
            .mimeTypeForFile(e.name, QMimeDatabase::MatchExtension).comment();
    case ByDateGroups: {
        const qint64 days = e.modified.daysTo(QDateTime::currentDateTime());
        if (days <= 0) return QStringLiteral("Today");
        if (days == 1) return QStringLiteral("Yesterday");
        if (days < 7) return QStringLiteral("Previous 7 days");
        if (days < 30) return QStringLiteral("Previous 30 days");
        return QLocale().toString(e.modified, QStringLiteral("MMMM yyyy"));
    }
    case BySizeGroups:
        if (e.isDir) return QStringLiteral("Folders");
        if (e.size == 0) return QStringLiteral("Empty");
        if (e.size < 100 * 1024) return QStringLiteral("Tiny");
        if (e.size < 10 * 1024 * 1024) return QStringLiteral("Small");
        if (e.size < 1024LL * 1024 * 1024) return QStringLiteral("Large");
        return QStringLiteral("Huge");
    }
    return {};
}

bool Directory::keeps(const DirEntry &e) const {
    if (e.isHidden && !m_showHidden)
        return false;
    if (!m_filter.isEmpty() && !e.name.contains(m_filter, Qt::CaseInsensitive))
        return false;
    if (!e.isDir && m_foldersOnly)
        return false;
    if (!e.isDir && !m_globs.isEmpty()) {
        for (const QRegularExpression &glob : m_globs)
            if (glob.match(e.name).hasMatch())
                return true;
        return false;
    }
    return true;
}

// What is inside a folder opened in place, and inside anything opened within it.
void Directory::appendExpanded(QVector<DirEntry> &rows, const QString &folder, int depth) const {
    if (depth > 16)
        return;
    // Read once and kept, so that filtering, sorting and hiding rebuild from memory rather than
    // going back to the disk on the thread that is drawing.
    auto cached = m_inside.constFind(folder);
    if (cached == m_inside.cend()) {
        QVector<DirEntry> inside = scan(folder);
        std::sort(inside.begin(), inside.end(), [](const DirEntry &a, const DirEntry &b) {
            if (a.isDir != b.isDir) return a.isDir;
            return a.name.compare(b.name, Qt::CaseInsensitive) < 0;
        });
        for (DirEntry &e : inside)
            e.path = folder + QLatin1Char('/') + e.name;
        cached = m_inside.insert(folder, inside);
    }

    for (DirEntry e : *cached) {
        if (!keeps(e))
            continue;
        e.depth = depth;
        rows.append(e);
        if (e.isDir && m_expanded.contains(e.path))
            appendExpanded(rows, e.path, depth + 1);
    }
}

void Directory::setStatus(Status status, const QString &error) {
    if (m_status == status && m_error == error)
        return;
    m_status = status;
    m_error = error;
    emit statusChanged();
}
