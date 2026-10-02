#pragma once

#include <QObject>
#include <QSet>
#include <QQmlEngine>
#include <QStringList>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>

#include <atomic>
#include <memory>
#include <string>
#include <thread>
#include <unordered_map>
#include <vector>

// What a volume or folder holds, counted once through and kept as a tree of folders (D87).
class DiskUsage : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

    Q_PROPERTY(QString root READ root NOTIFY stateChanged)
    Q_PROPERTY(bool scanning READ scanning NOTIFY stateChanged)
    // Whether a finished scan is there to be shown.
    Q_PROPERTY(bool ready READ ready NOTIFY stateChanged)
    Q_PROPERTY(qint64 files READ files NOTIFY progressChanged)
    Q_PROPERTY(qint64 bytes READ bytes NOTIFY progressChanged)
    // Folders the scan was not allowed into.
    Q_PROPERTY(int unreadable READ unreadable NOTIFY stateChanged)
    // How long the last scan took, in milliseconds.
    Q_PROPERTY(int elapsed READ elapsed NOTIFY stateChanged)
    // What each kind index means, in order.
    Q_PROPERTY(QStringList kindNames READ kindNames CONSTANT)
    // Bumped whenever the tree changes, so a binding that calls into it can depend on something.
    Q_PROPERTY(int generation READ generation NOTIFY changed)

public:
    struct Node;
    struct Top {
        std::string path;
        qint64 size = 0;
        qint64 mtime = 0;
        int kind = 0;
    };
    struct Result;

    explicit DiskUsage(QObject *parent = nullptr);
    ~DiskUsage() override;

    QString root() const { return m_root; }
    bool scanning() const { return m_scanning; }
    bool ready() const { return bool(m_tree); }
    qint64 files() const { return m_files.load(); }
    qint64 bytes() const { return m_bytes.load(); }
    int unreadable() const { return m_unreadable; }
    int elapsed() const { return m_elapsed; }
    int generation() const { return m_generation; }
    QStringList kindNames() const;

    Q_INVOKABLE void scan(const QString &path);
    Q_INVOKABLE void cancel();
    // Counts these folders again, in the background.
    Q_INVOKABLE void refresh(const QStringList &paths);
    // Takes something that is no longer there out of the tree, and says whether it is gone. A
    // folder's size is the tree's own; a file's is the one given, since the scan kept no record of it.
    Q_INVOKABLE bool forget(const QString &path, qint64 size);

    // Every list below is of rows { name, path, size, dir, files, newest, kind, skipped, other }:
    // `kind` the kind most of its bytes are, `newest` the latest change in seconds.

    // One folder of the tree as a row with `kinds`, its bytes by kind, or empty when it was not scanned.
    Q_INVOKABLE QVariantMap node(const QString &path) const;
    // What a folder holds, largest first: its folders from the tree and its files read now. Past
    // `limit` the rest is one row with `other` set.
    Q_INVOKABLE QVariantList children(const QString &path, int limit) const;
    // The largest files the scan met under a folder, largest first.
    Q_INVOKABLE QVariantList largest(const QString &under, int limit) const;
    // The folders under one where the space actually is, largest first.
    Q_INVOKABLE QVariantList hotspots(const QString &under, int limit) const;
    // The folders under one where nothing has changed in `days`: { size, items }.
    Q_INVOKABLE QVariantMap untouched(const QString &under, int days, int limit) const;
    // Matches cleanup rules against the tree. Each rule is { paths, safe, beside, unless, pattern,
    // olderThan, endings, files, except }: `paths` are globs from / or ~, or from the scan's root when they
    // start with **, where * is one name and ** any depth. A match is kept when a name in `beside`
    // sits next to it, the `unless` path ({match} and {name} filled in) does not exist, its name
    // fits `pattern` and no glob in `except`, nothing in it changed in `olderThan` days, and a file
    // ends in one of `endings`; `files` lets the last name match files as well as folders. Nothing under .git, a
    // protected place, or a folder that cannot be written is matched, and of two matches of the
    // same safety one inside or at the other, only the outer, or the earlier rule's, is kept. Rows carry `rule`, the rule's
    // index, and `captures`, what each * matched.
    Q_INVOKABLE QVariantList evaluate(const QVariantList &rules) const;
    // A place trashing from here must never offer: the root, a volume, home and its standard folders.
    Q_INVOKABLE bool isProtected(const QString &path) const;

signals:
    void stateChanged();
    void progressChanged();
    void changed();

private:
    void start(std::vector<std::string> folders, bool whole);
    void finish(Result *result);
    Node *find(const QString &path) const;

    QString m_root;
    bool m_scanning = false;
    int m_unreadable = 0;
    int m_elapsed = 0;
    int m_generation = 0;
    std::atomic<qint64> m_files { 0 };
    std::atomic<qint64> m_bytes { 0 };

    std::unique_ptr<Node> m_tree;
    // Every folder by its name, for rules that look anywhere; made again after the tree changes.
    mutable std::unordered_map<std::string, std::vector<const Node *>> m_byName;
    mutable int m_byNameGeneration = -1;
    // What isProtected refuses by name, gathered when a scan starts.
    QSet<QString> m_protected;
    std::vector<Top> m_top;

    // The walk runs on a thread of its own, which runs the workers; a later scan cancels it first.
    std::thread m_thread;
    std::shared_ptr<std::atomic_bool> m_cancelled;
    int m_ticket = 0;
    QTimer m_tick;
};
