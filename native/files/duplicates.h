#pragma once

#include <QtGlobal>

#include <atomic>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

#include <sys/types.h>

// Finding files that are the same inside, for the Cleaner's Duplicates page (D90).
namespace duplicates {

// A file the scan met that could have a copy: one of the person's own, of some size.
struct File {
    std::string path;
    qint64 size = 0;
    // Allocated bytes, which is what removing a copy gives back.
    qint64 bytes = 0;
    qint64 mtime = 0;
    dev_t dev = 0;
    ino_t ino = 0;
};

// The smallest file worth comparing.
constexpr qint64 Smallest = 1 << 20;

struct Group {
    std::vector<File> files;
    // How many distinct sets of blocks the copies have: copies that share their blocks through a
    // reflink are one set, and removing one of them gives nothing back.
    int distinct = 0;
};

// Hashes already worked out, by file identity and state, so a second search reads only what changed.
struct Cache {
    std::mutex mutex;
    std::unordered_map<std::string, std::string> edges;
    std::unordered_map<std::string, std::string> whole;
};

// Groups of two or more files with the same contents. `total` is set to the bytes that have to be
// read in full once the cheap passes are done, and `done` counts them as they are read.
std::vector<Group> find(std::vector<File> files, const std::atomic_bool &cancelled,
                        std::atomic<qint64> &done, std::atomic<qint64> &total, Cache &cache);

} // namespace duplicates
