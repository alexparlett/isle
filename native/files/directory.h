#pragma once

#include <QAbstractListModel>
#include <QCollator>
#include <QDateTime>
#include <QFutureWatcher>
#include <QQmlEngine>
#include <QSocketNotifier>
#include <QRegularExpression>
#include <QHash>
#include <QSet>
#include <QString>
#include <QTimer>
#include <QVector>

struct DirEntry {
    QString name;
    // How far in the row sits when a list nests; zero for a row of the folder itself.
    int depth = 0;
    // Where the row is, for a row that was named rather than found in a folder. Empty otherwise,
    // since a folder's rows hang off the folder.
    QString path;
    QString iconName;
    // What the type is called in words, as "PNG image" rather than "image/png".
    QString kindName;
    qint64 size = 0;
    QDateTime modified;
    bool isDir = false;
    bool isSymlink = false;
    bool isHidden = false;
};

// One directory as a list model. The scan runs off the GUI thread; sorting, filtering and hiding
// are done over what it returned, so none of the three touches the disk again.
class Directory : public QAbstractListModel {
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QString path READ path WRITE setPath NOTIFY pathChanged)
    // Named paths to list instead of a folder's contents, for things that are a gathering rather
    // than a place: what was opened lately, what a search found. Nothing is watched while it is set.
    Q_PROPERTY(QStringList paths READ paths WRITE setPaths NOTIFY pathsChanged)
    Q_PROPERTY(bool showHidden READ showHidden WRITE setShowHidden NOTIFY showHiddenChanged)
    Q_PROPERTY(Sort sort READ sort WRITE setSort NOTIFY sortChanged)
    Q_PROPERTY(Qt::SortOrder sortOrder READ sortOrder WRITE setSortOrder NOTIFY sortOrderChanged)
    Q_PROPERTY(QString filter READ filter WRITE setFilter NOTIFY filterChanged)
    // Globs a file must match to be listed, as a file chooser's filter asks. Folders always list.
    Q_PROPERTY(QStringList patterns READ patterns WRITE setPatterns NOTIFY patternsChanged)
    // Narrowing a search the way Finder's do: to one family of file, and to what has changed since.
    // The kind is the first half of an icon theme name — "image", "audio", "video", "text",
    // "application" — or "folder" for folders. Empty shows everything.
    Q_PROPERTY(QString kind READ kind WRITE setKind NOTIFY kindChanged)
    Q_PROPERTY(QDateTime since READ since WRITE setSince NOTIFY sinceChanged)
    // Whether only folders are listed, for a chooser asking for one.
    Q_PROPERTY(bool foldersOnly READ foldersOnly WRITE setFoldersOnly NOTIFY foldersOnlyChanged)
    // Rows carry the heading they belong under when set, so a view can show them in groups.
    Q_PROPERTY(Grouping grouping READ grouping WRITE setGrouping NOTIFY groupingChanged)
    Q_PROPERTY(Status status READ status NOTIFY statusChanged)
    Q_PROPERTY(QString error READ error NOTIFY statusChanged)
    Q_PROPERTY(int count READ count NOTIFY countChanged)
    Q_PROPERTY(int total READ total NOTIFY countChanged)

public:
    enum Grouping { NoGroups, ByKindGroups, ByDateGroups, BySizeGroups };
    Q_ENUM(Grouping)

    enum Roles {
        NameRole = Qt::UserRole + 1,
        PathRole,
        IconNameRole,
        SizeRole,
        ModifiedRole,
        IsDirRole,
        IsSymlinkRole,
        IsHiddenRole,
        KindRole,
        GroupRole,
        DepthRole,
        ExpandedRole,
    };

    enum Sort { ByName, BySize, ByModified, ByKind };
    Q_ENUM(Sort)

    enum Status { Idle, Loading, Ready, Error };
    Q_ENUM(Status)

    explicit Directory(QObject *parent = nullptr);
    ~Directory() override;

    int rowCount(const QModelIndex &parent = {}) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;

    QString path() const { return m_path; }
    void setPath(const QString &path);
    QStringList paths() const { return m_paths; }
    void setPaths(const QStringList &paths);
    bool showHidden() const { return m_showHidden; }
    void setShowHidden(bool on);
    Sort sort() const { return m_sort; }
    void setSort(Sort sort);
    Qt::SortOrder sortOrder() const { return m_sortOrder; }
    void setSortOrder(Qt::SortOrder order);
    QString filter() const { return m_filter; }
    void setFilter(const QString &filter);
    QStringList patterns() const { return m_patterns; }
    void setPatterns(const QStringList &patterns);
    QString kind() const { return m_kind; }
    void setKind(const QString &kind);
    QDateTime since() const { return m_since; }
    void setSince(const QDateTime &since);
    bool foldersOnly() const { return m_foldersOnly; }
    void setFoldersOnly(bool on);
    Grouping grouping() const { return m_grouping; }
    void setGrouping(Grouping grouping);
    Status status() const { return m_status; }
    QString error() const { return m_error; }
    int count() const { return int(m_rows.size()); }
    int total() const { return int(m_all.size()); }

    // Re-reads the directory. The watcher calls it; a view wants it for a manual refresh.
    Q_INVOKABLE void refresh();
    // The row's full path, for a delegate that has only its index.
    Q_INVOKABLE QString pathAt(int row) const;
    // The row a name sits on after the current sort, or -1. Naming a file after a rename selects it.
    Q_INVOKABLE int rowOf(const QString &name) const;
    // Whether the row is a folder, for a delegate deciding what a double click means.
    Q_INVOKABLE bool isDirAt(int row) const;
    // The first row at or after `from` whose name starts with this, for typing a name to jump to it.
    Q_INVOKABLE int startingWith(const QString &prefix, int from) const;
    // Which row a path is at, or -1. Names repeat once folders are opened in place; paths do not.
    Q_INVOKABLE int rowOfPath(const QString &path) const;
    // The longest name in the listing, for sizing a column to what it holds.
    Q_INVOKABLE QString longestName() const;
    // The heading a row sits under, so a view can tell where one run of them ends and the next starts.
    Q_INVOKABLE QString groupAt(int row) const;
    // Open a folder in place, as a list that nests does, or shut it again.
    Q_INVOKABLE void expand(int row);
    Q_INVOKABLE void collapse(int row);

signals:
    void pathChanged();
    void pathsChanged();
    void showHiddenChanged();
    void sortChanged();
    void sortOrderChanged();
    void filterChanged();
    void patternsChanged();
    void kindChanged();
    void sinceChanged();
    void foldersOnlyChanged();
    void groupingChanged();
    void statusChanged();
    void countChanged();

private:
    void startScan();
    void scanFinished();
    void rebuild();
    void setStatus(Status status, const QString &error = {});
    QString groupOf(const DirEntry &entry) const;
    void appendExpanded(QVector<DirEntry> &rows, const QString &folder, int depth) const;
    // Where a row is. A nested row carries its own path; a top-level row is named inside the folder
    // the listing came from, which during a navigation is not yet the folder being navigated to.
    QString pathOf(const DirEntry &entry) const;
    // Whether a row survives the hidden, filter, pattern and folders-only settings.
    bool keeps(const DirEntry &entry) const;

    QString m_path;
    QStringList m_paths;
    // The folder the rows on screen came from, to tell a first look from a second.
    QString m_scannedPath;
    bool m_showHidden = false;
    Sort m_sort = ByName;
    Qt::SortOrder m_sortOrder = Qt::AscendingOrder;
    QString m_filter;
    QStringList m_patterns;
    QString m_kind;
    QDateTime m_since;
    QList<QRegularExpression> m_globs;
    bool m_foldersOnly = false;
    Grouping m_grouping = NoGroups;
    Status m_status = Idle;
    QString m_error;

    QVector<DirEntry> m_all;
    QVector<DirEntry> m_rows;
    // Folders opened in place, by path, so they stay open across a rescan.
    QSet<QString> m_expanded;
    // What is inside each opened folder, so a rebuild does not go back to the disk for it.
    mutable QHash<QString, QVector<DirEntry>> m_inside;
    // The folder the rows were built from and which folders were open at the time, so a rebuild
    // that would look the same but mean something else is never skipped.
    QString m_rowsBase;
    QSet<QString> m_rowsExpanded;

    // A scan that finishes after the path has moved on belongs to a directory nobody is looking at.
    quint64 m_generation = 0;
    QFutureWatcher<QVector<DirEntry>> m_watcher;
    quint64 m_watchedGeneration = 0;

    // Qt's own watcher reports a directory's entries coming and going but not a file in it being
    // written, which is half of what a listing shows. inotify on the directory reports both.
    void watch(const QString &path);
    void unwatch();
    int m_inotify = -1;
    int m_watch = -1;
    QSocketNotifier *m_notifier = nullptr;

    QTimer m_settle;
    QCollator m_collator;
};
