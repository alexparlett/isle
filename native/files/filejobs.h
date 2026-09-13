#pragma once

#include <QMutex>
#include <QObject>
#include <QQmlEngine>
#include <QStringList>
#include <QThread>
#include <QWaitCondition>

#include <atomic>

// One copy, move, trash, delete, rename or new folder, running off the GUI thread. A job that meets
// something already there stops and asks, and goes on when the answer arrives.
class FileJob : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("Jobs come from FileJobs.")

    Q_PROPERTY(Kind kind READ kind CONSTANT)
    Q_PROPERTY(State state READ state NOTIFY stateChanged)
    Q_PROPERTY(qreal progress READ progress NOTIFY progressChanged)
    // The name being worked on, for a line that says what is happening.
    Q_PROPERTY(QString current READ current NOTIFY progressChanged)
    Q_PROPERTY(int count READ count NOTIFY progressChanged)
    Q_PROPERTY(int total READ total NOTIFY progressChanged)
    Q_PROPERTY(QString error READ error NOTIFY stateChanged)
    // The name that already exists at the destination, while the job is asking about it.
    Q_PROPERTY(QString conflictName READ conflictName NOTIFY conflictChanged)

public:
    enum Kind { Copy, Move, Trash, Delete, Rename, NewFolder, Restore, Extract, Compress, RenameMany, Duplicate };
    Q_ENUM(Kind)

    enum State { Running, Asking, Done, Failed, Cancelled };
    Q_ENUM(State)

    enum Answer { Replace, Skip, Keep };
    Q_ENUM(Answer)

    FileJob(Kind kind, const QStringList &sources, const QString &destination, QObject *parent = nullptr);
    ~FileJob() override;

    Kind kind() const { return m_kind; }
    State state() const { return m_state; }
    qreal progress() const { return m_bytesTotal > 0 ? qreal(m_bytesDone) / qreal(m_bytesTotal) : 0; }
    QString current() const { return m_current; }
    int count() const { return m_count; }
    int total() const { return m_sources.size(); }
    QString error() const { return m_error; }
    QString conflictName() const { return m_conflictName; }

    // Replace what is there, skip this one, or keep both by giving the new one another name.
    Q_INVOKABLE void answer(Answer answer, bool forAll = false);
    Q_INVOKABLE void cancel();

    // What would put this job back, or nothing when it cannot be undone.
    Q_INVOKABLE bool undoable() const;

    void start();

signals:
    void stateChanged();
    void progressChanged();
    void conflictChanged();
    void finished(FileJob *job);

private:
    friend class FileJobs;
    void run();
    void setState(State state, const QString &error = {});
    void report(const QString &name, int count);
    // Asks the GUI thread and waits for the answer. Returns what to do with this one file.
    Answer ask(const QString &name);

    bool copyTree(const QString &source, const QString &destination);
    bool copyFile(const QString &source, const QString &destination);
    bool moveOne(const QString &source, const QString &destination);
    bool trashOne(const QString &path);
    bool removeTree(const QString &path);
    // The destination for a source, having asked about it if something is there already.
    QString placeFor(const QString &source, bool *skip);

    Kind m_kind;
    QStringList m_sources;
    QString m_destination;
    // Restore only: where each source goes back to, one for one with m_sources.
    QStringList m_targets;

    State m_state = Running;
    QString m_error;
    QString m_current;
    QString m_conflictName;
    int m_count = 0;
    qint64 m_bytesDone = 0;
    qint64 m_bytesTotal = 0;

    std::atomic_bool m_cancelled { false };
    QMutex m_mutex;
    QWaitCondition m_answered;
    Answer m_answer = Skip;
    bool m_answerForAll = false;
    bool m_haveAnswer = false;

    // What the job did, so it can be put back: pairs of where a thing ended up and where it was.
    QStringList m_undoFrom;
    QStringList m_undoTo;

    QThread *m_thread = nullptr;
};

// Starts jobs and remembers the last one, so it can be put back.
class FileJobs : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

    Q_PROPERTY(QList<FileJob *> running READ running NOTIFY runningChanged)
    Q_PROPERTY(bool canUndo READ canUndo NOTIFY canUndoChanged)
    // What undoing would put back, for a menu item that says so.
    Q_PROPERTY(QString undoLabel READ undoLabel NOTIFY canUndoChanged)
    // Where the trash keeps what it holds, so a view can list it like any other folder.
    Q_PROPERTY(QString trashPath READ trashPath CONSTANT)

public:
    explicit FileJobs(QObject *parent = nullptr);

    QList<FileJob *> running() const { return m_running; }
    bool canUndo() const { return m_undo.kind != -1; }
    QString undoLabel() const;

    Q_INVOKABLE FileJob *copy(const QStringList &sources, const QString &destination);
    Q_INVOKABLE FileJob *move(const QStringList &sources, const QString &destination);
    Q_INVOKABLE FileJob *trash(const QStringList &paths);
    Q_INVOKABLE FileJob *remove(const QStringList &paths);
    Q_INVOKABLE FileJob *rename(const QString &path, const QString &name);
    // A copy beside the original, named the way a second of something is named.
    Q_INVOKABLE FileJob *duplicate(const QStringList &paths);
    Q_INVOKABLE FileJob *newFolder(const QString &parent, const QString &name);
    // Unpacked beside the archive, in a folder of its own named after it.
    Q_INVOKABLE FileJob *extract(const QString &archive);
    // Packed into one file in the folder they came from. The name decides the format bsdtar writes.
    Q_INVOKABLE FileJob *compress(const QStringList &paths, const QString &name);
    // One name for many, "#" standing for the number, the ending of each kept as it was.
    Q_INVOKABLE FileJob *renameMany(const QStringList &paths, const QString &pattern);
    // Whether a name is one bsdtar is likely to be able to unpack.
    Q_INVOKABLE bool isArchive(const QString &path) const;

    QString trashPath() const;
    // Everything in the trash put back where it came from, and the trash emptied.
    Q_INVOKABLE FileJob *restoreFromTrash(const QStringList &paths);
    Q_INVOKABLE FileJob *emptyTrash();
    Q_INVOKABLE FileJob *undo();

signals:
    void runningChanged();
    void canUndoChanged();
    void jobFinished(FileJob *job);

private:
    FileJob *begin(FileJob *job);
    void retire(FileJob *job);

    QList<FileJob *> m_running;
    struct Undo {
        int kind = -1;
        QStringList from;
        QStringList to;
    } m_undo;
};
