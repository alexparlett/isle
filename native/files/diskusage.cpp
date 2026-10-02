#include "diskusage.h"
#include "kinds.h"

#include <QDateTime>
#include <QDir>
#include <QElapsedTimer>
#include <QRegularExpression>
#include <QFile>
#include <QFileInfo>
#include <QStandardPaths>
#include <QStorageInfo>

#include <algorithm>
#include <array>
#include <condition_variable>
#include <cstring>
#include <deque>
#include <fstream>
#include <functional>
#include <mutex>
#include <set>
#include <sstream>
#include <unordered_set>

#include <dirent.h>
#include <fnmatch.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

struct DiskUsage::Node {
    std::string name;
    Node *parent = nullptr;
    std::vector<std::unique_ptr<Node>> dirs;
    // Bytes on the disk (allocated, not apparent) and files directly inside, then with everything below.
    qint64 own = 0;
    qint64 ownFiles = 0;
    qint64 size = 0;
    qint64 files = 0;
    // The latest modification anywhere below, in seconds.
    qint64 newest = 0;
    // Bytes below by kinds::Kind.
    std::array<qint64, kinds::Count> kinds {};
    // A mount the walk does not enter: another disk, a pseudo filesystem, or snapshots.
    bool skipped = false;
    bool unreadable = false;
    bool gone = false;
};

struct DiskUsage::Result {
    int ticket = 0;
    // A scan of the root, rather than some of its folders counted again.
    bool whole = false;
    std::vector<std::string> folders;
    std::vector<std::unique_ptr<Node>> trees;
    std::vector<Top> top;
    int unreadable = 0;
    qint64 elapsed = 0;
    bool cancelled = false;
};

namespace {

// How many of the largest files each walk keeps.
constexpr size_t TopKept = 500;

std::string joinPath(const std::string &dir, const std::string &name) {
    return dir == "/" ? "/" + name : dir + "/" + name;
}

// mountinfo writes a space in a path as \040.
std::string unescape(const std::string &s) {
    std::string out;
    for (size_t i = 0; i < s.size(); ++i) {
        if (s[i] == '\\' && i + 3 < s.size()) {
            out += char(std::stoi(s.substr(i + 1, 3), nullptr, 8));
            i += 3;
        } else {
            out += s[i];
        }
    }
    return out;
}

bool under(const std::string &path, const std::string &dir) {
    if (dir == "/")
        return true;
    return path == dir || (path.size() > dir.size() && path.compare(0, dir.size(), dir) == 0 && path[dir.size()] == '/');
}

// The mount points below `root` the walk stays out of. It crosses into a mount of the same device
// and filesystem, which is how a btrfs subvolume like @home appears, and nowhere else; snapshots
// share their blocks with what they copy, so counting them would count the disk twice over.
std::unordered_set<std::string> fencedMounts(const std::string &root) {
    struct Mount { std::string point, fstype, source; };
    std::vector<Mount> mounts;
    std::ifstream in("/proc/self/mountinfo");
    for (std::string line; std::getline(in, line);) {
        std::istringstream fields(line);
        std::vector<std::string> f;
        for (std::string w; fields >> w;)
            f.push_back(w);
        const auto dash = std::find(f.begin(), f.end(), "-");
        if (f.size() < 5 || dash == f.end() || f.end() - dash < 3)
            continue;
        mounts.push_back({ unescape(f[4]), *(dash + 1), *(dash + 2) });
    }

    const Mount *own = nullptr;
    for (const Mount &m : mounts)
        if (under(root, m.point) && (!own || m.point.size() >= own->point.size()))
            own = &m;

    std::unordered_set<std::string> fenced;
    for (const Mount &m : mounts) {
        if (m.point == root || !under(m.point, root))
            continue;
        const bool snapshots = m.point.size() >= 10 && m.point.compare(m.point.size() - 10, 10, ".snapshots") == 0;
        if (!own || m.source != own->source || m.fstype != own->fstype || snapshots)
            fenced.insert(m.point);
    }
    return fenced;
}

// Reads a set of folders through with a pool of threads, each folder by exactly one of them, so a
// folder's own fields and its list of children are only ever written by the thread reading it.
class Walker {
public:
    Walker(std::unordered_set<std::string> fenced, const std::atomic_bool &cancelled,
           std::atomic<qint64> &files, std::atomic<qint64> &bytes)
        : m_fenced(std::move(fenced)), m_cancelled(cancelled), m_files(files), m_bytes(bytes) {}

    void run(const std::vector<std::pair<DiskUsage::Node *, std::string>> &roots) {
        for (const auto &r : roots)
            m_queue.push_back({ r.first, r.second, kinds::ofPath(r.second) });
        m_pending = int(m_queue.size());
        const int count = std::clamp(int(std::thread::hardware_concurrency()), 4, 16);
        std::vector<std::thread> threads;
        for (int i = 0; i < count; ++i)
            threads.emplace_back([this] { work(); });
        for (std::thread &t : threads)
            t.join();
    }

    std::vector<DiskUsage::Top> top;
    std::atomic_int unreadable { 0 };

private:
    struct Work {
        DiskUsage::Node *node = nullptr;
        std::string path;
        // What the folders above said everything under them is, or kinds::None.
        int kind = kinds::None;
    };

    void work() {
        std::vector<DiskUsage::Top> heap;
        for (;;) {
            Work item;
            {
                std::unique_lock lock(m_mutex);
                m_ready.wait(lock, [this] { return !m_queue.empty() || m_pending == 0; });
                if (m_queue.empty())
                    break;
                item = std::move(m_queue.front());
                m_queue.pop_front();
            }
            if (!m_cancelled)
                read(item, heap);
            std::lock_guard lock(m_mutex);
            if (--m_pending == 0)
                m_ready.notify_all();
        }
        std::lock_guard lock(m_mutex);
        top.insert(top.end(), heap.begin(), heap.end());
    }

    static bool smaller(const DiskUsage::Top &a, const DiskUsage::Top &b) { return a.size > b.size; }

    void keep(std::vector<DiskUsage::Top> &heap, const std::string &path, qint64 size, qint64 mtime, int kind) {
        if (heap.size() < TopKept) {
            heap.push_back({ path, size, mtime, kind });
            std::push_heap(heap.begin(), heap.end(), smaller);
        } else if (size > heap.front().size) {
            std::pop_heap(heap.begin(), heap.end(), smaller);
            heap.back() = { path, size, mtime, kind };
            std::push_heap(heap.begin(), heap.end(), smaller);
        }
    }

    // A file with more than one name is counted at the first of them the walk meets.
    bool firstSight(dev_t dev, ino_t ino) {
        std::lock_guard lock(m_inodesMutex);
        return m_inodes.insert({ dev, ino }).second;
    }

    void read(const Work &item, std::vector<DiskUsage::Top> &heap) {
        DiskUsage::Node *node = item.node;
        const std::string &path = item.path;
        const int fd = open(path.c_str(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        DIR *dir = fd < 0 ? nullptr : fdopendir(fd);
        if (!dir) {
            if (fd >= 0)
                close(fd);
            if (errno == ENOENT) {
                node->gone = true;
            } else {
                node->unreadable = true;
                ++unreadable;
            }
            return;
        }

        std::vector<Work> found;
        qint64 own = 0, count = 0, newest = 0;
        while (dirent *e = readdir(dir)) {
            const char *name = e->d_name;
            if (name[0] == '.' && (name[1] == 0 || (name[1] == '.' && name[2] == 0)))
                continue;
            bool isDir = e->d_type == DT_DIR;
            struct stat st;
            if (!isDir) {
                if (fstatat(dirfd(dir), name, &st, AT_SYMLINK_NOFOLLOW) != 0)
                    continue;
                isDir = S_ISDIR(st.st_mode);
            }
            if (isDir) {
                auto child = std::make_unique<DiskUsage::Node>();
                child->name = name;
                child->parent = node;
                std::string childPath = joinPath(path, name);
                // Snapper keeps its snapshots as subvolumes under .snapshots whether or not it is mounted.
                if (m_fenced.count(childPath) || std::strcmp(name, ".snapshots") == 0)
                    child->skipped = true;
                else
                    found.push_back({ child.get(), std::move(childPath), kinds::ofFolder(name, path == "/", item.kind) });
                node->dirs.push_back(std::move(child));
                continue;
            }
            if (st.st_nlink > 1 && !firstSight(st.st_dev, st.st_ino))
                continue;
            const qint64 size = qint64(st.st_blocks) * 512;
            const int kind = item.kind != kinds::None ? item.kind : kinds::ofFile(name);
            node->kinds[kind] += size;
            own += size;
            ++count;
            newest = std::max<qint64>(newest, st.st_mtim.tv_sec);
            if (S_ISREG(st.st_mode) && (heap.size() < TopKept || size > heap.front().size))
                keep(heap, joinPath(path, name), size, st.st_mtim.tv_sec, kind);
        }
        closedir(dir);

        node->own = own;
        node->ownFiles = count;
        node->newest = newest;
        m_files += count;
        m_bytes += own;

        if (found.empty())
            return;
        std::lock_guard lock(m_mutex);
        m_pending += int(found.size());
        for (Work &w : found)
            m_queue.push_back(std::move(w));
        m_ready.notify_all();
    }

    std::unordered_set<std::string> m_fenced;
    const std::atomic_bool &m_cancelled;
    std::atomic<qint64> &m_files;
    std::atomic<qint64> &m_bytes;

    std::mutex m_mutex;
    std::condition_variable m_ready;
    std::deque<Work> m_queue;
    int m_pending = 0;

    std::mutex m_inodesMutex;
    std::set<std::pair<dev_t, ino_t>> m_inodes;
};

void total(DiskUsage::Node *n) {
    n->size = n->own;
    n->files = n->ownFiles;
    for (auto &c : n->dirs) {
        total(c.get());
        n->size += c->size;
        n->files += c->files;
        n->newest = std::max(n->newest, c->newest);
        for (int k = 0; k < kinds::Count; ++k)
            n->kinds[k] += c->kinds[k];
    }
}

using Kinds = std::array<qint64, kinds::Count>;

// Carries a change below a folder into it and every folder above it.
void adjust(DiskUsage::Node *n, qint64 size, qint64 files, const Kinds &by) {
    for (; n; n = n->parent) {
        n->size += size;
        n->files += files;
        for (int k = 0; k < kinds::Count; ++k)
            n->kinds[k] += by[k];
    }
}

Kinds minus(const Kinds &a, const Kinds &b) {
    Kinds out;
    for (int k = 0; k < kinds::Count; ++k)
        out[k] = a[k] - b[k];
    return out;
}

void sortTop(std::vector<DiskUsage::Top> &top) {
    std::sort(top.begin(), top.end(), [](const auto &a, const auto &b) { return a.size > b.size; });
    if (top.size() > TopKept)
        top.resize(TopKept);
}

} // namespace

DiskUsage::DiskUsage(QObject *parent) : QObject(parent) {
    m_tick.setInterval(150);
    connect(&m_tick, &QTimer::timeout, this, &DiskUsage::progressChanged);
}

DiskUsage::~DiskUsage() {
    if (m_cancelled)
        *m_cancelled = true;
    if (m_thread.joinable())
        m_thread.join();
}

void DiskUsage::scan(const QString &path) {
    const QString target = QDir::cleanPath(QFileInfo(path.isEmpty() ? QDir::homePath() : path).absoluteFilePath());
    cancel();
    m_root = target;
    const QString home = QDir::homePath();
    m_protected = { QStringLiteral("/"), home };
    for (const auto where : { QStandardPaths::DesktopLocation, QStandardPaths::DocumentsLocation,
                              QStandardPaths::DownloadLocation, QStandardPaths::MusicLocation,
                              QStandardPaths::PicturesLocation, QStandardPaths::MoviesLocation,
                              QStandardPaths::TemplatesLocation, QStandardPaths::PublicShareLocation,
                              QStandardPaths::GenericConfigLocation, QStandardPaths::GenericDataLocation,
                              QStandardPaths::GenericCacheLocation })
        m_protected.insert(QStandardPaths::writableLocation(where));
    for (const char *kept : { "/.local", "/.ssh", "/.gnupg", "/.var", "/.var/app" })
        m_protected.insert(home + QLatin1String(kept));
    for (const QStorageInfo &volume : QStorageInfo::mountedVolumes())
        m_protected.insert(volume.rootPath());
    m_tree.reset();
    m_top.clear();
    m_unreadable = 0;
    m_files = 0;
    m_bytes = 0;
    ++m_generation;
    start({ QFile::encodeName(target).toStdString() }, true);
    emit changed();
}

void DiskUsage::cancel() {
    ++m_ticket;
    if (m_cancelled)
        *m_cancelled = true;
    if (m_thread.joinable())
        m_thread.join();
    if (m_scanning) {
        m_scanning = false;
        m_tick.stop();
        emit stateChanged();
        emit progressChanged();
    }
}

void DiskUsage::refresh(const QStringList &paths) {
    if (m_scanning || !m_tree)
        return;
    std::vector<std::string> folders;
    for (const QString &p : paths) {
        if (!find(p))
            continue;
        const std::string f = QFile::encodeName(QDir::cleanPath(p)).toStdString();
        if (std::none_of(folders.begin(), folders.end(), [&](const std::string &o) { return under(f, o); })) {
            std::erase_if(folders, [&](const std::string &o) { return under(o, f); });
            folders.push_back(f);
        }
    }
    if (!folders.empty())
        start(std::move(folders), false);
}

void DiskUsage::start(std::vector<std::string> folders, bool whole) {
    // A refresh waits for the one before it rather than cancelling it, since both are wanted.
    if (m_thread.joinable())
        m_thread.join();
    m_cancelled = std::make_shared<std::atomic_bool>(false);
    auto result = std::make_shared<Result>();
    result->ticket = m_ticket;
    result->whole = whole;
    result->folders = std::move(folders);
    if (whole) {
        m_scanning = true;
        m_tick.start();
        emit stateChanged();
    }

    const std::string root = QFile::encodeName(m_root).toStdString();
    auto cancelled = m_cancelled;
    m_thread = std::thread([this, result, cancelled, root] {
        QElapsedTimer clock;
        clock.start();
        std::atomic<qint64> files { 0 }, bytes { 0 };
        Walker walker(fencedMounts(root), *cancelled, result->whole ? m_files : files, result->whole ? m_bytes : bytes);
        std::vector<std::pair<Node *, std::string>> roots;
        for (const std::string &f : result->folders) {
            auto node = std::make_unique<Node>();
            node->name = f;
            roots.emplace_back(node.get(), f);
            result->trees.push_back(std::move(node));
        }
        walker.run(roots);
        for (auto &t : result->trees)
            total(t.get());
        result->top = std::move(walker.top);
        sortTop(result->top);
        result->unreadable = walker.unreadable;
        result->elapsed = clock.elapsed();
        result->cancelled = *cancelled;
        QMetaObject::invokeMethod(this, [this, result] { finish(result.get()); }, Qt::QueuedConnection);
    });
}

void DiskUsage::finish(Result *result) {
    if (result->ticket != m_ticket || result->cancelled)
        return;
    if (m_thread.joinable())
        m_thread.join();

    if (result->whole) {
        m_tree = std::move(result->trees.front());
        m_tree->name = QFile::encodeName(m_root).toStdString();
        m_top = std::move(result->top);
        m_unreadable = result->unreadable;
        m_elapsed = int(result->elapsed);
        m_files = m_tree->files;
        m_bytes = m_tree->size;
        m_scanning = false;
        m_tick.stop();
    } else {
        for (size_t i = 0; i < result->folders.size(); ++i) {
            const std::string &path = result->folders[i];
            Node *target = find(QFile::decodeName(QByteArray::fromStdString(path)));
            if (!target)
                continue;
            Node *fresh = result->trees[i].get();
            if (fresh->gone && target->parent) {
                Node *parent = target->parent;
                const qint64 size = target->size, files = target->files;
                const Kinds by = minus({}, target->kinds);
                std::erase_if(parent->dirs, [&](const auto &c) { return c.get() == target; });
                adjust(parent, -size, -files, by);
                m_bytes -= size;
                m_files -= files;
            } else {
                const qint64 size = fresh->size - target->size, files = fresh->files - target->files;
                const Kinds by = minus(fresh->kinds, target->kinds);
                target->dirs = std::move(fresh->dirs);
                for (auto &c : target->dirs)
                    c->parent = target;
                target->own = fresh->own;
                target->ownFiles = fresh->ownFiles;
                target->newest = fresh->newest;
                target->unreadable = fresh->unreadable;
                adjust(target, size, files, by);
                m_bytes += size;
                m_files += files;
            }
            std::erase_if(m_top, [&](const Top &t) { return under(t.path, path); });
        }
        m_top.insert(m_top.end(), result->top.begin(), result->top.end());
        sortTop(m_top);
    }
    ++m_generation;
    emit changed();
    emit stateChanged();
    emit progressChanged();
}

bool DiskUsage::forget(const QString &path, qint64 size) {
    const QString p = QDir::cleanPath(path);
    const std::string raw = QFile::encodeName(p).toStdString();
    struct stat st;
    if (lstat(raw.c_str(), &st) == 0)
        return false;
    if (Node *n = find(p); n && n->parent) {
        Node *parent = n->parent;
        const qint64 gone = n->size, files = n->files;
        const Kinds by = minus({}, n->kinds);
        std::erase_if(parent->dirs, [&](const auto &c) { return c.get() == n; });
        adjust(parent, -gone, -files, by);
        m_bytes -= gone;
        m_files -= files;
    } else if (Node *parent = find(QFileInfo(p).absolutePath())) {
        const int folder = kinds::ofPath(QFile::encodeName(QFileInfo(p).absolutePath()).toStdString());
        Kinds by {};
        by[folder != kinds::None ? folder : kinds::ofFile(QFile::encodeName(QFileInfo(p).fileName()).constData())] = -size;
        parent->own = std::max<qint64>(0, parent->own - size);
        parent->ownFiles = std::max<qint64>(0, parent->ownFiles - 1);
        adjust(parent, -size, -1, by);
        m_bytes -= size;
        m_files -= 1;
    } else {
        return true;
    }
    std::erase_if(m_top, [&](const Top &t) { return under(t.path, raw); });
    ++m_generation;
    emit changed();
    emit progressChanged();
    return true;
}

DiskUsage::Node *DiskUsage::find(const QString &path) const {
    if (!m_tree)
        return nullptr;
    const QString p = QDir::cleanPath(path);
    if (p == m_root)
        return m_tree.get();
    const QString prefix = m_root == QLatin1String("/") ? m_root : m_root + QLatin1Char('/');
    if (!p.startsWith(prefix))
        return nullptr;
    Node *n = m_tree.get();
    for (const QString &part : p.mid(prefix.size()).split(QLatin1Char('/'), Qt::SkipEmptyParts)) {
        const std::string name = QFile::encodeName(part).toStdString();
        const auto it = std::find_if(n->dirs.begin(), n->dirs.end(), [&](const auto &c) { return c->name == name; });
        if (it == n->dirs.end())
            return nullptr;
        n = it->get();
    }
    return n;
}

namespace {

int dominant(const Kinds &by) {
    return int(std::max_element(by.begin(), by.end()) - by.begin());
}

QString pathOf(const DiskUsage::Node *n) {
    std::vector<const std::string *> names;
    for (; n->parent; n = n->parent)
        names.push_back(&n->name);
    std::string path = n->name;
    for (auto it = names.rbegin(); it != names.rend(); ++it)
        path = joinPath(path, **it);
    return QFile::decodeName(QByteArray::fromStdString(path));
}

QVariantMap folderRow(const DiskUsage::Node *n) {
    return {
        { QStringLiteral("name"), QFile::decodeName(QByteArray::fromStdString(n->name)) },
        { QStringLiteral("path"), pathOf(n) },
        { QStringLiteral("size"), n->size },
        { QStringLiteral("dir"), true },
        { QStringLiteral("files"), n->files },
        { QStringLiteral("newest"), n->newest },
        { QStringLiteral("kind"), dominant(n->kinds) },
        { QStringLiteral("skipped"), n->skipped },
        { QStringLiteral("other"), false },
    };
}

} // namespace

QStringList DiskUsage::kindNames() const { return kinds::names(); }

QVariantMap DiskUsage::node(const QString &path) const {
    const Node *n = find(path);
    if (!n)
        return {};
    QVariantMap row = folderRow(n);
    const QString p = QDir::cleanPath(path);
    row[QStringLiteral("path")] = p;
    row[QStringLiteral("name")] = p == m_root ? p : QFileInfo(p).fileName();
    QVariantList by;
    for (qint64 bytes : n->kinds)
        by.append(bytes);
    row[QStringLiteral("kinds")] = by;
    return row;
}

QVariantList DiskUsage::children(const QString &path, int limit) const {
    const Node *n = find(path);
    if (!n)
        return {};
    const QString dirPath = QDir::cleanPath(path);
    const QByteArray raw = QFile::encodeName(dirPath);
    std::vector<QVariantMap> rows;
    for (const auto &c : n->dirs) {
        QVariantMap row = folderRow(c.get());
        row[QStringLiteral("path")] = dirPath == QLatin1String("/") ? QLatin1Char('/') + row[QStringLiteral("name")].toString()
                                                                    : dirPath + QLatin1Char('/') + row[QStringLiteral("name")].toString();
        rows.push_back(std::move(row));
    }

    // Files are not kept by the scan, so they are read as the folder is opened.
    const int folder = kinds::ofPath(raw.toStdString());
    const int fd = open(raw.constData(), O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (DIR *dir = fd < 0 ? nullptr : fdopendir(fd)) {
        while (dirent *e = readdir(dir)) {
            if (e->d_type == DT_DIR)
                continue;
            struct stat st;
            // A link holds nothing of its own; what it points at is counted where it lives.
            if (fstatat(dirfd(dir), e->d_name, &st, AT_SYMLINK_NOFOLLOW) != 0 || S_ISDIR(st.st_mode) || S_ISLNK(st.st_mode))
                continue;
            const QString name = QFile::decodeName(e->d_name);
            rows.push_back({
                { QStringLiteral("name"), name },
                { QStringLiteral("path"), dirPath == QLatin1String("/") ? QLatin1Char('/') + name : dirPath + QLatin1Char('/') + name },
                { QStringLiteral("size"), qint64(st.st_blocks) * 512 },
                { QStringLiteral("dir"), false },
                { QStringLiteral("files"), 1 },
                { QStringLiteral("newest"), qint64(st.st_mtim.tv_sec) },
                { QStringLiteral("kind"), folder != kinds::None ? folder : kinds::ofFile(e->d_name) },
                { QStringLiteral("skipped"), false },
                { QStringLiteral("other"), false },
            });
        }
        closedir(dir);
    } else if (fd >= 0) {
        close(fd);
    }

    const auto skipped = [](const QVariantMap &r) { return r.value(QStringLiteral("skipped")).toBool(); };
    const auto size = [](const QVariantMap &r) { return r.value(QStringLiteral("size")).toLongLong(); };
    std::sort(rows.begin(), rows.end(), [&](const QVariantMap &a, const QVariantMap &b) {
        return skipped(a) != skipped(b) ? skipped(b) : size(a) > size(b);
    });
    QVariantList out;
    qint64 restSize = 0, restCount = 0;
    for (size_t i = 0; i < rows.size(); ++i) {
        if (int(i) < limit || skipped(rows[i])) {
            out.append(rows[i]);
        } else {
            restSize += size(rows[i]);
            ++restCount;
        }
    }
    if (restCount)
        out.append(QVariantMap {
            { QStringLiteral("name"), QString() },
            { QStringLiteral("path"), QString() },
            { QStringLiteral("size"), restSize },
            { QStringLiteral("dir"), false },
            { QStringLiteral("files"), restCount },
            { QStringLiteral("newest"), 0 },
            { QStringLiteral("kind"), int(kinds::Other) },
            { QStringLiteral("skipped"), false },
            { QStringLiteral("other"), true },
        });
    return out;
}

QVariantList DiskUsage::largest(const QString &underPath, int limit) const {
    const std::string dir = QFile::encodeName(QDir::cleanPath(underPath)).toStdString();
    QVariantList out;
    for (const Top &t : m_top) {
        if (out.size() >= limit)
            break;
        if (!under(t.path, dir))
            continue;
        const QString path = QFile::decodeName(QByteArray::fromStdString(t.path));
        out.append(QVariantMap {
            { QStringLiteral("name"), QFileInfo(path).fileName() },
            { QStringLiteral("path"), path },
            { QStringLiteral("size"), t.size },
            { QStringLiteral("dir"), false },
            { QStringLiteral("files"), 1 },
            { QStringLiteral("newest"), t.mtime },
            { QStringLiteral("kind"), t.kind },
            { QStringLiteral("skipped"), false },
            { QStringLiteral("other"), false },
        });
    }
    return out;
}

QVariantList DiskUsage::hotspots(const QString &underPath, int limit) const {
    const Node *root = find(underPath);
    if (!root)
        return {};
    // A folder most of whose space is in one child is only the way to that child, so the walk goes on
    // past it; one whose space is spread out is where the space is.
    const qint64 floor = std::max<qint64>(root->size / 100, 1 << 20);
    std::vector<const Node *> found;
    std::function<void(const Node *)> visit = [&](const Node *n) {
        if (n->skipped || n->size < floor)
            return;
        const Node *biggest = nullptr;
        for (const auto &c : n->dirs)
            if (!biggest || c->size > biggest->size)
                biggest = c.get();
        if (biggest && biggest->size * 2 >= n->size) {
            for (const auto &c : n->dirs)
                visit(c.get());
            return;
        }
        found.push_back(n);
    };
    for (const auto &c : root->dirs)
        visit(c.get());
    std::sort(found.begin(), found.end(), [](const Node *a, const Node *b) { return a->size > b->size; });
    QVariantList out;
    for (size_t i = 0; i < found.size() && int(i) < limit; ++i)
        out.append(folderRow(found[i]));
    return out;
}

QVariantMap DiskUsage::untouched(const QString &underPath, int days, int limit) const {
    const Node *root = find(underPath);
    if (!root)
        return {};
    const qint64 cutoff = QDateTime::currentSecsSinceEpoch() - qint64(days) * 86400;
    qint64 total = 0;
    std::vector<const Node *> found;
    // The system's files keep their packages' dates, which says nothing about whether they are used.
    std::function<void(const Node *)> visit = [&](const Node *n) {
        if (n->skipped || n->size == 0 || dominant(n->kinds) == kinds::System)
            return;
        if (n->newest > 0 && n->newest < cutoff) {
            total += n->size;
            found.push_back(n);
            return;
        }
        for (const auto &c : n->dirs)
            visit(c.get());
    };
    visit(root);
    std::sort(found.begin(), found.end(), [](const Node *a, const Node *b) { return a->size > b->size; });
    QVariantList items;
    for (size_t i = 0; i < found.size() && int(i) < limit; ++i)
        items.append(folderRow(found[i]));
    return { { QStringLiteral("size"), total }, { QStringLiteral("items"), items } };
}

namespace {

struct Match {
    int rule = 0;
    bool safe = false;
    std::string path;
    std::string name;
    qint64 size = 0;
    qint64 newest = 0;
    int kind = 0;
    bool dir = false;
    QStringList captures;
};

std::vector<std::string> splitPath(const std::string &path) {
    std::vector<std::string> out;
    size_t start = 0;
    while (start <= path.size()) {
        size_t end = path.find('/', start);
        if (end == std::string::npos)
            end = path.size();
        if (end > start)
            out.push_back(path.substr(start, end - start));
        start = end + 1;
    }
    return out;
}

bool isGlob(const std::string &s) { return s.find_first_of("*?[") != std::string::npos; }

bool exists(const std::string &path) {
    struct stat st;
    return lstat(path.c_str(), &st) == 0;
}

// Walks the tree along one pattern's components, calling `found` with each folder it reaches at the
// end, and with the parent and the remaining glob when the last name may also be a file.
struct PatternWalk {
    const std::vector<std::string> &parts;
    std::function<void(const DiskUsage::Node *, const std::string &, const QStringList &)> folder;
    std::function<void(const DiskUsage::Node *, const std::string &, const std::string &, const QStringList &)> files;
    bool withFiles = false;

    void walk(const DiskUsage::Node *n, const std::string &path, size_t i, const QStringList &caps) {
        if (i == parts.size()) {
            folder(n, path, caps);
            return;
        }
        const std::string &part = parts[i];
        if (part == "**") {
            walk(n, path, i + 1, caps);
            for (const auto &c : n->dirs)
                if (!c->skipped && c->name != ".git")
                    walk(c.get(), joinPath(path, c->name), i, caps);
            return;
        }
        if (i + 1 == parts.size() && withFiles)
            files(n, path, part, caps);
        if (!isGlob(part)) {
            for (const auto &c : n->dirs)
                if (c->name == part && !c->skipped)
                    walk(c.get(), joinPath(path, c->name), i + 1, caps);
            return;
        }
        for (const auto &c : n->dirs) {
            if (c->skipped || fnmatch(part.c_str(), c->name.c_str(), FNM_PERIOD) != 0)
                continue;
            QStringList more = caps;
            more.append(QFile::decodeName(QByteArray::fromStdString(c->name)));
            walk(c.get(), joinPath(path, c->name), i + 1, more);
        }
    }
};

std::string normalise(const std::string &path) {
    return QFile::encodeName(QDir::cleanPath(QFile::decodeName(QByteArray::fromStdString(path)))).toStdString();
}

} // namespace

QVariantList DiskUsage::evaluate(const QVariantList &rules) const {
    if (!m_tree)
        return {};
    const std::string home = QFile::encodeName(QDir::homePath()).toStdString();
    const std::string root = m_tree->name;
    const std::vector<std::string> rootParts = splitPath(root);
    const qint64 now = QDateTime::currentSecsSinceEpoch();
    std::vector<Match> found;

    for (int r = 0; r < rules.size(); ++r) {
        const QVariantMap rule = rules[r].toMap();
        const bool safe = rule.value(QStringLiteral("safe")).toBool();
        const bool withFiles = rule.value(QStringLiteral("files")).toBool();
        const QStringList beside = rule.value(QStringLiteral("beside")).toStringList();
        const QStringList unless = rule.value(QStringLiteral("unless")).toStringList();
        const QStringList endings = rule.value(QStringLiteral("endings")).toStringList();
        std::vector<std::string> except;
        for (const QString &e : rule.value(QStringLiteral("except")).toStringList())
            except.push_back(QFile::encodeName(e).toStdString());
        const QString patternText = rule.value(QStringLiteral("pattern")).toString();
        const QRegularExpression pattern(patternText);
        const int olderThan = rule.value(QStringLiteral("olderThan")).toInt();

        // Whether a candidate passes every condition but the ones about where it is.
        const auto keep = [&](const std::string &path, const std::string &name, qint64 newest, bool dir) {
            const std::string parent = path.substr(0, path.rfind('/'));
            if (std::any_of(except.begin(), except.end(), [&](const std::string &e) { return fnmatch(e.c_str(), name.c_str(), 0) == 0; }))
                return false;
            if (!beside.isEmpty() && std::none_of(beside.begin(), beside.end(), [&](const QString &b) {
                    return exists(joinPath(parent.empty() ? "/" : parent, QFile::encodeName(b).toStdString()));
                }))
                return false;
            for (const QString &u : unless) {
                QString t = u;
                t.replace(QStringLiteral("{match}"), QFile::decodeName(QByteArray::fromStdString(path)));
                t.replace(QStringLiteral("{name}"), QFile::decodeName(QByteArray::fromStdString(name)));
                if (exists(normalise(QFile::encodeName(t).toStdString())))
                    return false;
            }
            if (!patternText.isEmpty() && !pattern.match(QFile::decodeName(QByteArray::fromStdString(name))).hasMatch())
                return false;
            if (olderThan > 0 && newest >= now - qint64(olderThan) * 86400)
                return false;
            if (!dir && !endings.isEmpty()) {
                const QString lower = QFile::decodeName(QByteArray::fromStdString(name)).toLower();
                if (std::none_of(endings.begin(), endings.end(), [&](const QString &e) { return lower.endsWith(e.toLower()); }))
                    return false;
            }
            return access(parent.empty() ? "/" : parent.c_str(), W_OK) == 0;
        };

        for (const QString &globText : rule.value(QStringLiteral("paths")).toStringList()) {
            std::string glob = QFile::encodeName(globText).toStdString();
            if (glob.rfind("~/", 0) == 0)
                glob = home + glob.substr(1);
            std::vector<std::string> parts = splitPath(glob);
            // An absolute pattern has to pass through the scan's root to be found in the tree.
            if (glob.rfind("**", 0) != 0) {
                if (parts.size() < rootParts.size())
                    continue;
                bool through = true;
                for (size_t i = 0; i < rootParts.size() && through; ++i)
                    through = parts[i] == "**" ? false : fnmatch(parts[i].c_str(), rootParts[i].c_str(), 0) == 0;
                if (!through)
                    continue;
                parts.erase(parts.begin(), parts.begin() + long(rootParts.size()));
            }
            // Anywhere, then a name: start from every folder with that name rather than walking to them.
            std::vector<std::pair<const Node *, size_t>> starts { { m_tree.get(), 0 } };
            if (parts.size() > 1 && parts[0] == "**" && !isGlob(parts[1])) {
                if (m_byNameGeneration != m_generation) {
                    m_byName.clear();
                    std::function<void(const Node *)> index = [&](const Node *n) {
                        for (const auto &c : n->dirs) {
                            m_byName[c->name].push_back(c.get());
                            if (!c->skipped && c->name != ".git")
                                index(c.get());
                        }
                    };
                    index(m_tree.get());
                    m_byNameGeneration = m_generation;
                }
                starts.clear();
                if (const auto it = m_byName.find(parts[1]); it != m_byName.end())
                    for (const Node *n : it->second)
                        if (!n->skipped)
                            starts.push_back({ n, 2 });
            }
            PatternWalk walk { parts, {}, {}, withFiles };
            walk.folder = [&](const DiskUsage::Node *n, const std::string &path, const QStringList &caps) {
                if (n == m_tree.get() || n->size == 0 || !keep(path, n->name, n->newest, true))
                    return;
                found.push_back({ r, safe, path, n->name, n->size, n->newest, dominant(n->kinds), true, caps });
            };
            walk.files = [&](const DiskUsage::Node *, const std::string &dirPath, const std::string &part, const QStringList &caps) {
                DIR *dir = opendir(dirPath.c_str());
                if (!dir)
                    return;
                const int folderKind = kinds::ofPath(dirPath);
                while (dirent *e = readdir(dir)) {
                    if (e->d_type == DT_DIR || fnmatch(part.c_str(), e->d_name, FNM_PERIOD) != 0)
                        continue;
                    struct stat st;
                    if (fstatat(dirfd(dir), e->d_name, &st, AT_SYMLINK_NOFOLLOW) != 0 || !S_ISREG(st.st_mode))
                        continue;
                    const std::string path = joinPath(dirPath, e->d_name);
                    const qint64 size = qint64(st.st_blocks) * 512;
                    if (size == 0 || !keep(path, e->d_name, st.st_mtim.tv_sec, false))
                        continue;
                    QStringList more = caps;
                    more.append(QFile::decodeName(e->d_name));
                    found.push_back({ r, safe, path, e->d_name, size, st.st_mtim.tv_sec,
                                      folderKind != kinds::None ? folderKind : kinds::ofFile(e->d_name), false, more });
                }
                closedir(dir);
            };
            for (const auto &[node, from] : starts)
                walk.walk(node, from ? QFile::encodeName(pathOf(node)).toStdString() : root, from, {});
        }
    }

    // Outer before inner, so a match inside one already kept is dropped; of two at the same place,
    // the earlier rule's is kept, which lets a named rule come before a general one.
    std::sort(found.begin(), found.end(), [](const Match &a, const Match &b) { return a.path != b.path ? a.path < b.path : a.rule < b.rule; });
    QVariantList out;
    std::vector<const Match *> kept;
    for (const Match &m : found) {
        if (std::any_of(kept.begin(), kept.end(), [&](const Match *k) { return k->safe == m.safe && under(m.path, k->path); }))
            continue;
        const QString path = QFile::decodeName(QByteArray::fromStdString(m.path));
        if (isProtected(path) || m.path.find("/.git/") != std::string::npos)
            continue;
        kept.push_back(&m);
        out.append(QVariantMap {
            { QStringLiteral("rule"), m.rule },
            { QStringLiteral("name"), QFile::decodeName(QByteArray::fromStdString(m.name)) },
            { QStringLiteral("path"), path },
            { QStringLiteral("size"), m.size },
            { QStringLiteral("dir"), m.dir },
            { QStringLiteral("files"), m.dir ? find(path) ? find(path)->files : 0 : 1 },
            { QStringLiteral("newest"), m.newest },
            { QStringLiteral("kind"), m.kind },
            { QStringLiteral("skipped"), false },
            { QStringLiteral("other"), false },
            { QStringLiteral("captures"), m.captures },
        });
    }
    return out;
}

bool DiskUsage::isProtected(const QString &path) const {
    const QString p = QDir::cleanPath(path);
    return m_protected.contains(p) || QFileInfo(p).absolutePath() == QLatin1String("/") || QFileInfo(p).fileName() == QLatin1String(".git");
}
