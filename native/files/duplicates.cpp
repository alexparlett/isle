#include "duplicates.h"

#include <QCryptographicHash>

#include <algorithm>
#include <map>
#include <set>
#include <thread>

#include <fcntl.h>
#include <linux/fiemap.h>
#include <linux/fs.h>
#include <sys/ioctl.h>
#include <unistd.h>

namespace duplicates {

namespace {

// How much of each end a file's first comparison reads.
constexpr qint64 Edge = 64 * 1024;

std::string identity(const File &f) {
    return std::to_string(f.dev) + ':' + std::to_string(f.ino) + ':' + std::to_string(f.size) + ':' + std::to_string(f.mtime);
}

// The first and last Edge bytes, or the whole file when it is no longer than both.
std::string edges(const File &f) {
    const int fd = open(f.path.c_str(), O_RDONLY | O_CLOEXEC);
    if (fd < 0)
        return {};
    QCryptographicHash hash(QCryptographicHash::Blake2b_256);
    std::vector<char> buffer(Edge);
    const auto readAt = [&](off_t at) {
        const ssize_t n = pread(fd, buffer.data(), buffer.size(), at);
        if (n > 0)
            hash.addData(QByteArrayView(buffer.data(), n));
        return n >= 0;
    };
    const bool ok = readAt(0) && (f.size <= Edge * 2 || readAt(f.size - Edge));
    close(fd);
    return ok ? hash.result().toStdString() : std::string();
}

std::string whole(const File &f, const std::atomic_bool &cancelled, std::atomic<qint64> &done) {
    const int fd = open(f.path.c_str(), O_RDONLY | O_CLOEXEC);
    if (fd < 0)
        return {};
    posix_fadvise(fd, 0, 0, POSIX_FADV_SEQUENTIAL);
    QCryptographicHash hash(QCryptographicHash::Blake2b_256);
    std::vector<char> buffer(1 << 20);
    ssize_t n;
    while (!cancelled && (n = read(fd, buffer.data(), buffer.size())) > 0) {
        hash.addData(QByteArrayView(buffer.data(), n));
        done += n;
    }
    posix_fadvise(fd, 0, 0, POSIX_FADV_DONTNEED);
    close(fd);
    return cancelled || n < 0 ? std::string() : hash.result().toStdString();
}

// Where a file's first extent sits on the disk, or 0 when it cannot be said; two copies at the
// same place are a reflink, not two lots of space.
quint64 firstExtent(const File &f) {
    const int fd = open(f.path.c_str(), O_RDONLY | O_CLOEXEC);
    if (fd < 0)
        return 0;
    // The header and room for the one extent asked for, which the kernel writes after it.
    alignas(struct fiemap) char buffer[sizeof(struct fiemap) + sizeof(struct fiemap_extent)] {};
    auto *map = reinterpret_cast<struct fiemap *>(buffer);
    map->fm_length = FIEMAP_MAX_OFFSET;
    map->fm_extent_count = 1;
    const bool ok = ioctl(fd, FS_IOC_FIEMAP, map) == 0 && map->fm_mapped_extents > 0
        && !(map->fm_extents[0].fe_flags & (FIEMAP_EXTENT_NOT_ALIGNED | FIEMAP_EXTENT_DATA_INLINE | FIEMAP_EXTENT_UNKNOWN));
    close(fd);
    return ok ? map->fm_extents[0].fe_physical : 0;
}

// Runs `work` over 0..count on a pool of threads.
template <typename Work>
void spread(size_t count, Work work) {
    std::atomic<size_t> next { 0 };
    const int n = std::clamp(int(std::thread::hardware_concurrency()), 2, 8);
    std::vector<std::thread> threads;
    for (int i = 0; i < n; ++i)
        threads.emplace_back([&] {
            for (size_t k; (k = next++) < count;)
                work(k);
        });
    for (std::thread &t : threads)
        t.join();
}

// Splits each group by a key worked out per file, keeping the pieces of two or more.
template <typename Key>
std::vector<std::vector<File>> split(const std::vector<std::vector<File>> &groups, Key key) {
    std::vector<std::pair<size_t, const File *>> all;
    for (size_t g = 0; g < groups.size(); ++g)
        for (const File &f : groups[g])
            all.push_back({ g, &f });
    std::vector<std::string> keys(all.size());
    spread(all.size(), [&](size_t i) { keys[i] = key(*all[i].second); });

    std::map<std::pair<size_t, std::string>, std::vector<File>> pieces;
    for (size_t i = 0; i < all.size(); ++i)
        if (!keys[i].empty())
            pieces[{ all[i].first, keys[i] }].push_back(*all[i].second);
    std::vector<std::vector<File>> out;
    for (auto &[k, files] : pieces)
        if (files.size() > 1)
            out.push_back(std::move(files));
    return out;
}

} // namespace

std::vector<Group> find(std::vector<File> files, const std::atomic_bool &cancelled,
                        std::atomic<qint64> &done, std::atomic<qint64> &total, Cache &cache) {
    // Same size, more than one name for the same file counted once, and only where a copy could be
    // taken away.
    std::map<qint64, std::vector<File>> bySize;
    std::set<std::pair<dev_t, ino_t>> seen;
    std::map<std::string, bool> writable;
    for (File &f : files) {
        const std::string parent = f.path.substr(0, f.path.rfind('/'));
        const auto it = writable.try_emplace(parent, access(parent.c_str(), W_OK) == 0).first;
        if (it->second && seen.insert({ f.dev, f.ino }).second)
            bySize[f.size].push_back(std::move(f));
    }
    std::vector<std::vector<File>> groups;
    for (auto &[size, list] : bySize)
        if (list.size() > 1)
            groups.push_back(std::move(list));

    const auto cached = [&](std::unordered_map<std::string, std::string> &map, const File &f, auto make) {
        const std::string id = identity(f);
        {
            std::lock_guard lock(cache.mutex);
            if (const auto it = map.find(id); it != map.end())
                return it->second;
        }
        std::string h = make(f);
        if (!h.empty()) {
            std::lock_guard lock(cache.mutex);
            map[id] = h;
        }
        return h;
    };

    groups = split(groups, [&](const File &f) { return cancelled ? std::string() : cached(cache.edges, f, edges); });

    qint64 toRead = 0;
    for (const auto &g : groups)
        if (g.front().size > Edge * 2)
            for (const File &f : g) {
                std::lock_guard lock(cache.mutex);
                if (!cache.whole.count(identity(f)))
                    toRead += f.size;
            }
    total = toRead;
    groups = split(groups, [&](const File &f) {
        if (f.size <= Edge * 2)
            return std::string("edges");
        return cached(cache.whole, f, [&](const File &g) { return whole(g, cancelled, done); });
    });
    if (cancelled)
        return {};

    std::vector<Group> out;
    for (auto &g : groups) {
        std::set<quint64> places;
        int unknown = 0;
        for (const File &f : g) {
            const quint64 at = firstExtent(f);
            if (at)
                places.insert(at);
            else
                ++unknown;
        }
        std::sort(g.begin(), g.end(), [](const File &a, const File &b) { return a.mtime < b.mtime; });
        out.push_back({ std::move(g), int(places.size()) + unknown });
    }
    return out;
}

} // namespace duplicates
