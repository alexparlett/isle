#include "filejobs.h"

#include <QDateTime>
#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QFileInfo>
#include <QStandardPaths>
#include <QProcess>
#include <QUrl>

#include <errno.h>

namespace {

// The freedesktop trash for the home volume. A file on another volume would want that volume's own,
// which is a thing to add when there is a volume to try it on.
QString trashRoot() {
    return QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation) + QStringLiteral("/Trash");
}

// A name nothing else in the folder has, by putting a number before the extension the way every
// file manager does: "notes.txt" becomes "notes (2).txt".
QString freeName(const QString &folder, const QString &name) {
    const QFileInfo info(name);
    const QString base = info.completeBaseName();
    const QString suffix = info.suffix().isEmpty() ? QString() : QLatin1Char('.') + info.suffix();
    QDir dir(folder);
    for (int n = 2; n < 10000; ++n) {
        const QString candidate = QStringLiteral("%1 (%2)%3").arg(base).arg(n).arg(suffix);
        if (!dir.exists(candidate))
            return candidate;
    }
    return name;
}

// What bsdtar is asked to unpack. Anything else is a file like any other.
bool looksLikeArchive(const QString &name) {
    static const QStringList endings = {
        QStringLiteral(".tar"), QStringLiteral(".tar.gz"), QStringLiteral(".tgz"),
        QStringLiteral(".tar.bz2"), QStringLiteral(".tbz2"), QStringLiteral(".tar.xz"),
        QStringLiteral(".txz"), QStringLiteral(".tar.zst"), QStringLiteral(".zip"),
        QStringLiteral(".7z"), QStringLiteral(".rar"), QStringLiteral(".iso"),
    };
    for (const QString &ending : endings)
        if (name.endsWith(ending, Qt::CaseInsensitive))
            return true;
    return false;
}

// The name without whatever archive ending it has, so "photos.tar.gz" unpacks into "photos".
QString withoutArchiveEnding(const QString &name) {
    const int dot = name.indexOf(QLatin1Char('.'), 1);
    return dot < 0 ? name : name.left(dot);
}

qint64 sizeOf(const QString &path) {
    const QFileInfo info(path);
    if (!info.isDir() || info.isSymLink())
        return info.size();
    qint64 total = 0;
    QDirIterator it(path, QDir::AllEntries | QDir::Hidden | QDir::NoDotAndDotDot | QDir::System,
                    QDirIterator::Subdirectories);
    while (it.hasNext()) {
        it.next();
        total += it.fileInfo().isDir() ? 0 : it.fileInfo().size();
    }
    return total;
}

} // namespace

FileJob::FileJob(Kind kind, const QStringList &sources, const QString &destination, QObject *parent)
    : QObject(parent), m_kind(kind), m_sources(sources), m_destination(destination) {}

FileJob::~FileJob() {
    cancel();
    if (m_thread) {
        m_thread->quit();
        m_thread->wait();
    }
}

void FileJob::start() {
    m_thread = QThread::create([this] { run(); });
    connect(m_thread, &QThread::finished, this, [this] { emit finished(this); });
    m_thread->start();
}

void FileJob::cancel() {
    m_cancelled = true;
    // A job waiting on an answer will never see the flag until it is woken.
    QMutexLocker lock(&m_mutex);
    m_haveAnswer = true;
    m_answer = Skip;
    m_answered.wakeAll();
}

void FileJob::answer(Answer answer, bool forAll) {
    QMutexLocker lock(&m_mutex);
    m_answer = answer;
    m_answerForAll = forAll;
    m_haveAnswer = true;
    m_answered.wakeAll();
}

void FileJob::setState(State state, const QString &error) {
    if (m_state == state && m_error == error)
        return;
    m_state = state;
    m_error = error;
    QMetaObject::invokeMethod(this, [this] { emit stateChanged(); }, Qt::QueuedConnection);
}

void FileJob::report(const QString &name, int count) {
    m_current = name;
    m_count = count;
    QMetaObject::invokeMethod(this, [this] { emit progressChanged(); }, Qt::QueuedConnection);
}

FileJob::Answer FileJob::ask(const QString &name) {
    QMutexLocker lock(&m_mutex);
    if (m_answerForAll)
        return m_answer;

    m_conflictName = name;
    m_haveAnswer = false;
    setState(Asking);
    QMetaObject::invokeMethod(this, [this] { emit conflictChanged(); }, Qt::QueuedConnection);

    while (!m_haveAnswer && !m_cancelled)
        m_answered.wait(&m_mutex);

    m_conflictName.clear();
    setState(Running);
    return m_answer;
}

QString FileJob::placeFor(const QString &source, bool *skip) {
    *skip = false;
    const QString name = QFileInfo(source).fileName();
    QString target = QDir(m_destination).filePath(name);
    if (!QFileInfo::exists(target))
        return target;

    switch (ask(name)) {
    case Skip:
        *skip = true;
        return {};
    case Keep:
        return QDir(m_destination).filePath(freeName(m_destination, name));
    case Replace:
        return target;
    }
    return target;
}

bool FileJob::copyFile(const QString &source, const QString &destination) {
    QFile in(source);
    if (!in.open(QIODevice::ReadOnly))
        return false;
    QFile::remove(destination);
    QFile out(destination);
    if (!out.open(QIODevice::WriteOnly))
        return false;

    // Copied in pieces so the job can be stopped part way and can say how far it is.
    QByteArray buffer;
    buffer.resize(1 << 20);
    while (!in.atEnd()) {
        if (m_cancelled)
            return false;
        const qint64 read = in.read(buffer.data(), buffer.size());
        if (read <= 0)
            break;
        if (out.write(buffer.constData(), read) != read)
            return false;
        m_bytesDone += read;
        QMetaObject::invokeMethod(this, [this] { emit progressChanged(); }, Qt::QueuedConnection);
    }
    out.close();
    QFile::setPermissions(destination, QFile::permissions(source));
    return true;
}

bool FileJob::copyTree(const QString &source, const QString &destination) {
    const QFileInfo info(source);
    if (info.isSymLink())
        return QFile::link(info.symLinkTarget(), destination);
    if (!info.isDir())
        return copyFile(source, destination);

    if (!QDir().mkpath(destination))
        return false;
    QDirIterator it(source, QDir::AllEntries | QDir::Hidden | QDir::NoDotAndDotDot | QDir::System);
    while (it.hasNext()) {
        it.next();
        if (m_cancelled)
            return false;
        if (!copyTree(it.filePath(), QDir(destination).filePath(it.fileName())))
            return false;
    }
    return true;
}

bool FileJob::removeTree(const QString &path) {
    const QFileInfo info(path);
    if (info.isDir() && !info.isSymLink())
        return QDir(path).removeRecursively();
    return QFile::remove(path);
}

bool FileJob::moveOne(const QString &source, const QString &destination) {
    QFile::remove(destination);
    if (QFile::rename(source, destination))
        return true;
    // Another filesystem: rename cannot cross one, so it is a copy and then a delete.
    if (!copyTree(source, destination))
        return false;
    return removeTree(source);
}

bool FileJob::trashOne(const QString &path) {
    const QString root = trashRoot();
    if (!QDir().mkpath(root + QStringLiteral("/files")) || !QDir().mkpath(root + QStringLiteral("/info")))
        return false;

    const QString name = QFileInfo(path).fileName();
    QString target = name;
    if (QFileInfo::exists(root + QStringLiteral("/files/") + target))
        target = freeName(root + QStringLiteral("/files"), name);

    // The info file is written first, so nothing is ever in the trash without a note of where it came from.
    QFile info(root + QStringLiteral("/info/") + target + QStringLiteral(".trashinfo"));
    if (!info.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    info.write("[Trash Info]\n");
    info.write("Path=" + QUrl::toPercentEncoding(QFileInfo(path).absoluteFilePath(), "/") + "\n");
    info.write("DeletionDate=" + QDateTime::currentDateTime().toString(Qt::ISODate).toUtf8() + "\n");
    info.close();

    const QString grave = root + QStringLiteral("/files/") + target;
    if (!QFile::rename(path, grave)) {
        info.remove();
        return false;
    }
    m_undoFrom.append(grave);
    m_undoTo.append(path);
    return true;
}

void FileJob::run() {
    if (m_kind == Rename) {
        const QString source = m_sources.value(0);
        const QString target = QDir(QFileInfo(source).absolutePath()).filePath(m_destination);
        if (QFileInfo::exists(target)) {
            setState(Failed, QStringLiteral("There is already something called that"));
            return;
        }
        if (!QFile::rename(source, target)) {
            setState(Failed, QStringLiteral("Could not rename it"));
            return;
        }
        m_undoFrom.append(target);
        m_undoTo.append(source);
        setState(Done);
        return;
    }

    if (m_kind == NewFolder) {
        const QString target = QDir(m_destination).filePath(m_sources.value(0));
        if (!QDir().mkdir(target)) {
            setState(Failed, QStringLiteral("Could not make the folder"));
            return;
        }
        m_undoFrom.append(target);
        m_undoTo.append(QString());
        setState(Done);
        return;
    }

    if (m_kind == Extract) {
        const QString archive = m_sources.value(0);
        const QFileInfo info(archive);
        QString into = QDir(info.absolutePath()).filePath(withoutArchiveEnding(info.fileName()));
        if (QFileInfo::exists(into))
            into = QDir(info.absolutePath()).filePath(freeName(info.absolutePath(), withoutArchiveEnding(info.fileName())));
        if (!QDir().mkpath(into)) {
            setState(Failed, QStringLiteral("Could not make a folder to unpack into"));
            return;
        }
        report(info.fileName(), 1);

        QProcess tar;
        tar.setWorkingDirectory(into);
        tar.start(QStringLiteral("bsdtar"), { QStringLiteral("-xf"), info.absoluteFilePath() });
        if (!tar.waitForFinished(-1) || tar.exitCode() != 0) {
            QDir(into).removeRecursively();
            setState(Failed, QStringLiteral("Could not unpack ") + info.fileName());
            return;
        }
        m_undoFrom.append(into);
        m_undoTo.append(QString());
        setState(Done);
        return;
    }

    if (m_kind == Compress) {
        if (m_sources.isEmpty()) {
            setState(Failed, QStringLiteral("Nothing to pack"));
            return;
        }
        const QString folder = QFileInfo(m_sources.first()).absolutePath();
        const QString target = QDir(folder).filePath(m_destination);
        report(m_destination, 1);

        // Named relative to the folder they are in, so the archive holds names and not whole paths.
        QStringList args { QStringLiteral("-caf"), target };
        for (const QString &source : std::as_const(m_sources))
            args.append(QFileInfo(source).fileName());

        QProcess tar;
        tar.setWorkingDirectory(folder);
        tar.start(QStringLiteral("bsdtar"), args);
        if (!tar.waitForFinished(-1) || tar.exitCode() != 0) {
            QFile::remove(target);
            setState(Failed, QStringLiteral("Could not pack them"));
            return;
        }
        m_undoFrom.append(target);
        m_undoTo.append(QString());
        setState(Done);
        return;
    }

    if (m_kind == RenameMany) {
        int n = 0;
        for (const QString &source : std::as_const(m_sources)) {
            if (m_cancelled) {
                setState(Cancelled);
                return;
            }
            const QFileInfo info(source);
            const QString suffix = info.suffix().isEmpty() ? QString() : QLatin1Char('.') + info.suffix();
            ++n;
            QString name = m_destination;
            // Without a place for the number every name would be the same one, and the second would
            // collide with the first; the number goes on the end instead.
            if (name.contains(QLatin1Char('#')))
                name.replace(QLatin1Char('#'), QString::number(n));
            else
                name += QLatin1Char(' ') + QString::number(n);
            const QString target = QDir(info.absolutePath()).filePath(name + suffix);
            report(info.fileName(), n);
            if (target == source)
                continue;
            if (QFileInfo::exists(target) || !QFile::rename(source, target)) {
                setState(Failed, QStringLiteral("Could not rename ") + info.fileName());
                return;
            }
            m_undoFrom.append(target);
            m_undoTo.append(source);
        }
        setState(Done);
        return;
    }

    if (m_kind == Duplicate) {
        int made = 0;
        for (const QString &source : std::as_const(m_sources)) {
            if (m_cancelled) { setState(Cancelled); return; }
            const QFileInfo info(source);
            const QString target = QDir(info.absolutePath()).filePath(freeName(info.absolutePath(), info.fileName()));
            report(info.fileName(), ++made);
            if (!copyTree(source, target)) {
                setState(Failed, QStringLiteral("Could not duplicate ") + info.fileName());
                return;
            }
            m_undoFrom.append(target);
            m_undoTo.append(QString());
        }
        setState(Done);
        return;
    }

    if (m_kind == Restore) {
        int back = 0;
        for (int i = 0; i < m_sources.size() && i < m_targets.size(); ++i) {
            if (m_cancelled) {
                setState(Cancelled);
                return;
            }
            const QString to = m_targets.at(i);
            report(QFileInfo(to).fileName(), ++back);
            QDir().mkpath(QFileInfo(to).absolutePath());
            if (!moveOne(m_sources.at(i), to)) {
                setState(Failed, QStringLiteral("Could not put back ") + QFileInfo(to).fileName());
                return;
            }
            // The trash keeps a note beside what it holds; putting the thing back takes the note too.
            const QString note = trashRoot() + QStringLiteral("/info/")
                + QFileInfo(m_sources.at(i)).fileName() + QStringLiteral(".trashinfo");
            QFile::remove(note);
        }
        setState(Done);
        return;
    }

    if (m_kind == Copy || m_kind == Move) {
        for (const QString &source : std::as_const(m_sources))
            m_bytesTotal += sizeOf(source);
    }

    int done = 0;
    for (const QString &source : std::as_const(m_sources)) {
        if (m_cancelled) {
            setState(Cancelled);
            return;
        }
        report(QFileInfo(source).fileName(), ++done);

        bool ok = true;
        switch (m_kind) {
        case Trash:
            ok = trashOne(source);
            break;
        case Delete:
            ok = removeTree(source);
            break;
        case Copy:
        case Move: {
            bool skip = false;
            const QString target = placeFor(source, &skip);
            if (skip)
                continue;
            if (m_cancelled) {
                setState(Cancelled);
                return;
            }
            if (m_kind == Copy) {
                ok = copyTree(source, target);
                if (ok) { m_undoFrom.append(target); m_undoTo.append(QString()); }
            } else {
                ok = moveOne(source, target);
                if (ok) { m_undoFrom.append(target); m_undoTo.append(source); }
            }
            break;
        }
        default:
            break;
        }

        if (!ok) {
            setState(Failed, QStringLiteral("Could not finish with ") + QFileInfo(source).fileName());
            return;
        }
    }

    setState(m_cancelled ? Cancelled : Done);
}

bool FileJob::undoable() const {
    // A job that failed or was stopped part way has still done part of it, and that part is exactly
    // what wants undoing.
    return (m_state == Done || m_state == Failed || m_state == Cancelled) && !m_undoFrom.isEmpty()
        && (m_kind == Move || m_kind == Trash || m_kind == Copy || m_kind == Rename
            || m_kind == NewFolder || m_kind == Extract || m_kind == Compress || m_kind == RenameMany
            || m_kind == Duplicate);
}

FileJobs::FileJobs(QObject *parent) : QObject(parent) {}

QString FileJobs::undoLabel() const {
    switch (m_undo.kind) {
    case FileJob::Move: return QStringLiteral("Undo move");
    case FileJob::Trash: return QStringLiteral("Put back");
    case FileJob::Copy: return QStringLiteral("Undo copy");
    case FileJob::Rename: return QStringLiteral("Undo rename");
    case FileJob::NewFolder: return QStringLiteral("Undo new folder");
    case FileJob::Extract: return QStringLiteral("Undo unpack");
    case FileJob::Compress: return QStringLiteral("Undo pack");
    case FileJob::RenameMany: return QStringLiteral("Undo rename");
    case FileJob::Duplicate: return QStringLiteral("Undo duplicate");
    default: return QStringLiteral("Undo");
    }
}

FileJob *FileJobs::begin(FileJob *job) {
    m_running.append(job);
    emit runningChanged();
    connect(job, &FileJob::finished, this, [this, job] { retire(job); });
    job->start();
    return job;
}

void FileJobs::retire(FileJob *job) {
    m_running.removeAll(job);
    emit runningChanged();

    if (job->undoable()) {
        m_undo = { int(job->kind()), job->m_undoFrom, job->m_undoTo };
        emit canUndoChanged();
    }
    emit jobFinished(job);
    job->deleteLater();
}

FileJob *FileJobs::copy(const QStringList &sources, const QString &destination) {
    return begin(new FileJob(FileJob::Copy, sources, destination, this));
}

FileJob *FileJobs::move(const QStringList &sources, const QString &destination) {
    return begin(new FileJob(FileJob::Move, sources, destination, this));
}

FileJob *FileJobs::trash(const QStringList &paths) {
    return begin(new FileJob(FileJob::Trash, paths, QString(), this));
}

FileJob *FileJobs::remove(const QStringList &paths) {
    return begin(new FileJob(FileJob::Delete, paths, QString(), this));
}

FileJob *FileJobs::rename(const QString &path, const QString &name) {
    return begin(new FileJob(FileJob::Rename, { path }, name, this));
}

FileJob *FileJobs::newFolder(const QString &parent, const QString &name) {
    return begin(new FileJob(FileJob::NewFolder, { name }, parent, this));
}

FileJob *FileJobs::duplicate(const QStringList &paths) {
    return begin(new FileJob(FileJob::Duplicate, paths, QString(), this));
}

FileJob *FileJobs::extract(const QString &archive) {
    return begin(new FileJob(FileJob::Extract, { archive }, QString(), this));
}

FileJob *FileJobs::compress(const QStringList &paths, const QString &name) {
    return begin(new FileJob(FileJob::Compress, paths, name, this));
}

FileJob *FileJobs::renameMany(const QStringList &paths, const QString &pattern) {
    return begin(new FileJob(FileJob::RenameMany, paths, pattern, this));
}

bool FileJobs::isArchive(const QString &path) const {
    return looksLikeArchive(QFileInfo(path).fileName());
}

FileJob *FileJobs::undo() {
    if (m_undo.kind == -1)
        return nullptr;
    const Undo undo = m_undo;
    m_undo = {};
    emit canUndoChanged();

    // Copying and making a folder are put back by taking away what they made; everything else by
    // moving each thing to where it came from.
    if (undo.kind == FileJob::Copy || undo.kind == FileJob::NewFolder
        || undo.kind == FileJob::Extract || undo.kind == FileJob::Compress
        || undo.kind == FileJob::Duplicate)
        return begin(new FileJob(FileJob::Delete, undo.from, QString(), this));

    // Everything else goes back where it came from, each thing to its own place.
    auto *job = new FileJob(FileJob::Restore, undo.from, QString(), this);
    job->m_targets = undo.to;
    return begin(job);
}
