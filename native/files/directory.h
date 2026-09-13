#pragma once

#include <QAbstractListModel>
#include <QCollator>
#include <QDateTime>
#include <QFileSystemWatcher>
#include <QFutureWatcher>
#include <QQmlEngine>
#include <QString>
#include <QTimer>
#include <QVector>

struct DirEntry {
    QString name;
    QString iconName;
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
    Q_PROPERTY(bool showHidden READ showHidden WRITE setShowHidden NOTIFY showHiddenChanged)
    Q_PROPERTY(Sort sort READ sort WRITE setSort NOTIFY sortChanged)
    Q_PROPERTY(Qt::SortOrder sortOrder READ sortOrder WRITE setSortOrder NOTIFY sortOrderChanged)
    Q_PROPERTY(QString filter READ filter WRITE setFilter NOTIFY filterChanged)
    Q_PROPERTY(Status status READ status NOTIFY statusChanged)
    Q_PROPERTY(QString error READ error NOTIFY statusChanged)
    Q_PROPERTY(int count READ count NOTIFY countChanged)
    Q_PROPERTY(int total READ total NOTIFY countChanged)

public:
    enum Roles {
        NameRole = Qt::UserRole + 1,
        PathRole,
        IconNameRole,
        SizeRole,
        ModifiedRole,
        IsDirRole,
        IsSymlinkRole,
        IsHiddenRole,
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
    bool showHidden() const { return m_showHidden; }
    void setShowHidden(bool on);
    Sort sort() const { return m_sort; }
    void setSort(Sort sort);
    Qt::SortOrder sortOrder() const { return m_sortOrder; }
    void setSortOrder(Qt::SortOrder order);
    QString filter() const { return m_filter; }
    void setFilter(const QString &filter);
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

signals:
    void pathChanged();
    void showHiddenChanged();
    void sortChanged();
    void sortOrderChanged();
    void filterChanged();
    void statusChanged();
    void countChanged();

private:
    void startScan();
    void scanFinished();
    void rebuild();
    void setStatus(Status status, const QString &error = {});

    QString m_path;
    bool m_showHidden = false;
    Sort m_sort = ByName;
    Qt::SortOrder m_sortOrder = Qt::AscendingOrder;
    QString m_filter;
    Status m_status = Idle;
    QString m_error;

    QVector<DirEntry> m_all;
    QVector<DirEntry> m_rows;

    // A scan that finishes after the path has moved on belongs to a directory nobody is looking at.
    quint64 m_generation = 0;
    QFutureWatcher<QVector<DirEntry>> m_watcher;
    quint64 m_watchedGeneration = 0;

    QFileSystemWatcher m_fsWatcher;
    QTimer m_settle;
    QCollator m_collator;
};
