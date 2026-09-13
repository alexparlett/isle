#include "filejobs.h"

#include <QDateTime>
#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QFileInfo>
#include <QStandardPaths>
#include <QProcess>
#include <QUrl>

#include <QStorageInfo>

#include <errno.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

namespace {

// The freedesktop trash for the home volume.
QString homeTrash() {
    return QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation) + QStringLiteral("/Trash");
}

// The trash a path belongs in. A rename cannot cross a filesystem, so a file on another volume goes
// to that volume's own trash rather than being copied the length of the disk to the home one. The
// spec allows the administrator's "$mount/.Trash/$uid", which must be sticky and not a link, and
// otherwise the user's own "$mount/.Trash-$uid".
QString trashFor(const QString &path) {
    const QStorageInfo home(QDir::homePath());
    const QStorageInfo where(QFileInfo(path).absolutePath());
    if (!where.isValid() || where.rootPath() == home.rootPath())
        return homeTrash();

    const QString mount = where.rootPath();
    const QString uid = QString::number(::getuid());
    const QFileInfo shared(mount + QStringLiteral("/.Trash"));
    if (shared.isDir() && !shared.isSymLink() && (shared.permissions() & QFileDevice::ExeOther)
        && (QFile::permissions(shared.absoluteFilePath()) & QFileDevice::WriteOther)) {
        struct stat st;
        if (::stat(QFile::encodeName(shared.absoluteFilePath()).constData(), &st) == 0 && (st.st_mode & S_ISVTX))
            return shared.absoluteFilePath() + QLatin1Char('/') + uid;
    }
    return mount + QStringLiteral("/.Trash-") + uid;
}

// Every trash there is: the home one, and one per volume that has been given one.
QStringList allTrashes() {
    QStringList out { homeTrash() };
    const QString uid = QString::number(::getuid());
    const QStorageInfo home(QDir::homePath());
    for (const QStorageInfo &volume : QStorageInfo::mountedVolumes()) {
        if (!volume.isValid() || !volume.isReady() || volume.isReadOnly())
            continue;
        if (volume.rootPath() == home.rootPath())
            continue;
        for (const QString &root : { volume.rootPath() + QStringLiteral("/.Trash/") + uid,
                                     volume.rootPath() + QStringLiteral("/.Trash-") + uid })
            if (QFileInfo(root + QStringLiteral("/files")).isDir())
                out.append(root);
    }
    return out;
}

// Where a name's ending starts: the first dot after the first character, so "photos.tar.gz" keeps
// both halves and ".bashrc" is a name rather than an ending. A folder has no ending at all.
int endingAt(const QString &name, bool isDir) {
    return isDir ? -1 : name.indexOf(QLatin1Char('.'), 1);
}

// A name nothing else in the folder has, by putting a number before the ending the way every file
// manager does: "notes.txt" becomes "notes (2).txt". Empty when there is no free name to be had.
QString freeName(const QString &folder, const QString &name, bool isDir = false) {
    const int dot = endingAt(name, isDir);
    const QString base = dot < 0 ? name : name.left(dot);
    const QString suffix = dot < 0 ? QString() : name.mid(dot);
    QDir dir(folder);
    for (int n = 2; n < 10000; ++n) {
        const QString candidate = QStringLiteral("%1 (%2)%3").arg(base).arg(n).arg(suffix);
        if (!dir.exists(candidate))
            return candidate;
    }
    return {};
}

// A name the trash has neither a copy of nor a note about, so the two never come apart.
QString freeTrashName(const QString &root, const QString &name, bool isDir) {
    const QDir files(root + QStringLiteral("/files"));
    const QDir notes(root + QStringLiteral("/info"));
    const auto taken = [&files, &notes](const QString &candidate) {
        return files.exists(candidate) || notes.exists(candidate + QStringLiteral(".trashinfo"));
    };
    if (!taken(name))
        return name;
    const int dot = endingAt(name, isDir);
    const QString base = dot < 0 ? name : name.left(dot);
    const QString suffix = dot < 0 ? QString() : name.mid(dot);
    for (int n = 2; n < 10000; ++n) {
        const QString candidate = QStringLiteral("%1 (%2)%3").arg(base).arg(n).arg(suffix);
        if (!taken(candidate))
            return candidate;
    }
    return {};
}

// Everything at a path, gone.
bool removeAll(const QString &path) {
    const QFileInfo info(path);
    if (info.isDir() && !info.isSymLink())
        return QDir(path).removeRecursively();
    return QFile::remove(path);
}

// Where a copy is written before it is anything: beside its destination, so it is on the same
// filesystem and going into place is one rename.
QString stagedName(const QString &destination) {
    return destination + QStringLiteral(".isle-part");
}

// Put what was staged where it belongs. Whatever is already there survives until the new thing is
// in place: a file goes over in one rename, a folder is moved aside and only then dropped.
bool swapIntoPlace(const QString &staged, const QString &destination) {
    const QByteArray from = QFile::encodeName(staged);
    const QByteArray to = QFile::encodeName(destination);
    if (!QFileInfo::exists(destination) || (!QFileInfo(destination).isDir() && !QFileInfo(staged).isDir()))
        return ::rename(from.constData(), to.constData()) == 0;

    const QString aside = destination + QStringLiteral(".isle-old");
    removeAll(aside);
    const QByteArray kept = QFile::encodeName(aside);
    if (::rename(to.constData(), kept.constData()) != 0)
        return false;
    if (::rename(from.constData(), to.constData()) != 0) {
        ::rename(kept.constData(), to.constData());
        return false;
    }
    removeAll(aside);
    return true;
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
    if (m_thread)
        m_thread->wait();
}

void FileJob::start() {
    m_thread = QThread::create([this] { run(); });
    // The thread outlives run() by a moment, so it deletes itself rather than being deleted here.
    connect(m_thread, &QThread::finished, m_thread, &QObject::deleteLater);
    connect(m_thread, &QThread::destroyed, this, [this] { m_thread = nullptr; });
    connect(m_thread, &QThread::finished, this, [this] { emit finished(this); });
    m_thread->start();
}

void FileJob::cancel() {
    m_cancelled = true;
    QMutexLocker lock(&m_mutex);
    // An unpacker is a child process and does not watch the flag; it is stopped outright.
    if (m_tar)
        m_tar->kill();
    // A job waiting on an answer will never see the flag until it is woken.
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

// The job runs on its own thread and everything it says is read on the GUI thread, so the fields
// behind those readers are only ever written there, inside the same call that signals the change.
void FileJob::setState(State state, const QString &error) {
    QMetaObject::invokeMethod(this, [this, state, error] {
        if (m_state == state && m_error == error)
            return;
        m_state = state;
        m_error = error;
        emit stateChanged();
    }, Qt::QueuedConnection);
}

void FileJob::report(const QString &name, int count) {
    QMetaObject::invokeMethod(this, [this, name, count] {
        m_current = name;
        m_count = count;
        emit progressChanged();
    }, Qt::QueuedConnection);
}

FileJob::Answer FileJob::ask(const QString &name) {
    QMutexLocker lock(&m_mutex);
    if (m_answerForAll)
        return m_answer;

    m_haveAnswer = false;
    QMetaObject::invokeMethod(this, [this, name] {
        m_conflictName = name;
        emit conflictChanged();
    }, Qt::QueuedConnection);
    setState(Asking);

    while (!m_haveAnswer && !m_cancelled)
        m_answered.wait(&m_mutex);

    QMetaObject::invokeMethod(this, [this] {
        m_conflictName.clear();
        emit conflictChanged();
    }, Qt::QueuedConnection);
    setState(Running);
    return m_answer;
}

QString FileJob::placeFor(const QString &source, bool *skip) {
    *skip = false;
    const QString name = QFileInfo(source).fileName();
    const bool isDir = QFileInfo(source).isDir();
    QString target = QDir(m_destination).filePath(name);
    if (!QFileInfo::exists(target))
        return target;

    // Pasting into the folder it came from is not a clash with another file: a copy takes the next
    // free name, and a move has nothing to do at all.
    if (QFileInfo(target).canonicalFilePath() == QFileInfo(source).canonicalFilePath()) {
        if (m_kind != Copy) {
            *skip = true;
            return {};
        }
        const QString free = freeName(m_destination, name, isDir);
        if (free.isEmpty()) { *skip = true; return {}; }
        return QDir(m_destination).filePath(free);
    }

    switch (ask(name)) {
    case Skip:
        *skip = true;
        return {};
    case Keep: {
        const QString free = freeName(m_destination, name, isDir);
        if (free.isEmpty()) { *skip = true; return {}; }
        return QDir(m_destination).filePath(free);
    }
    case Replace:
        // What was there is about to be gone, and no undo can bring it back.
        m_replaced = true;
        return target;
    }
    return target;
}

bool FileJob::copyFile(const QString &source, const QString &destination) {
    QFile in(source);
    if (!in.open(QIODevice::ReadOnly))
        return false;
    // Written beside the destination and moved over it only once it is whole, so a failure part way
    // through never leaves a truncated file where a complete one was.
    const QString staged = stagedName(destination);
    QFile::remove(staged);
    QFile out(staged);
    if (!out.open(QIODevice::WriteOnly))
        return false;

    // Copied in pieces so the job can be stopped part way and can say how far it is.
    QByteArray buffer;
    buffer.resize(1 << 20);
    bool whole = true;
    while (!in.atEnd()) {
        if (m_cancelled) { whole = false; break; }
        const qint64 read = in.read(buffer.data(), buffer.size());
        if (read <= 0)
            break;
        if (out.write(buffer.constData(), read) != read) { whole = false; break; }
        m_bytesDone += read;
        QMetaObject::invokeMethod(this, [this] { emit progressChanged(); }, Qt::QueuedConnection);
    }
    whole = whole && out.flush();
    out.close();
    if (!whole) {
        QFile::remove(staged);
        return false;
    }
    QFile::setPermissions(staged, QFile::permissions(source));
    if (!swapIntoPlace(staged, destination)) {
        QFile::remove(staged);
        return false;
    }
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
    if (!info.isDir() || info.isSymLink())
        return QFile::remove(path);

    // Walked rather than handed to removeRecursively, which cannot be stopped part way.
    QDirIterator it(path, QDir::AllEntries | QDir::Hidden | QDir::NoDotAndDotDot | QDir::System);
    while (it.hasNext()) {
        it.next();
        if (m_cancelled)
            return false;
        if (!removeTree(it.filePath()))
            return false;
    }
    return QDir().rmdir(path);
}

bool FileJob::moveOne(const QString &source, const QString &destination) {
    if (source == destination)
        return true;
    // rename replaces a file in one step and never leaves the destination missing; what it will not
    // do is cross a filesystem, or go over a folder that already exists.
    if (::rename(QFile::encodeName(source).constData(), QFile::encodeName(destination).constData()) == 0)
        return true;
    const int why = errno;
    if (why != EXDEV && why != EEXIST && why != ENOTEMPTY && why != EISDIR && why != ENOTDIR)
        return false;
    if (!copyTree(source, destination))
        return false;
    return removeTree(source);
}

bool FileJob::trashOne(const QString &path) {
    const QString root = trashFor(path);
    if (!QDir().mkpath(root + QStringLiteral("/files")) || !QDir().mkpath(root + QStringLiteral("/info")))
        return false;

    const QString name = QFileInfo(path).fileName();
    const QString target = freeTrashName(root, name, QFileInfo(path).isDir());
    if (target.isEmpty())
        return false;

    // The info file is written first, so nothing is ever in the trash without a note of where it came from.
    QFile info(root + QStringLiteral("/info/") + target + QStringLiteral(".trashinfo"));
    if (!info.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    // A volume's own trash records where a thing came from relative to that volume, so the note
    // still says where to put it back after the volume is mounted somewhere else.
    const QString mount = QStorageInfo(QFileInfo(path).absolutePath()).rootPath();
    const QString absolute = QFileInfo(path).absoluteFilePath();
    const bool onVolume = root != homeTrash() && mount != QLatin1String("/")
        && absolute.startsWith(mount + QLatin1Char('/'));
    const QString written = onVolume ? absolute.mid(mount.size() + 1) : absolute;

    info.write("[Trash Info]\n");
    info.write("Path=" + QUrl::toPercentEncoding(written, "/") + "\n");
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

    if (m_kind == NewFile) {
        const QString name = QFileInfo::exists(QDir(m_destination).filePath(m_sources.value(0)))
            ? freeName(m_destination, m_sources.value(0)) : m_sources.value(0);
        const QString target = QDir(m_destination).filePath(name);
        QFile made(target);
        if (name.isEmpty() || !made.open(QIODevice::WriteOnly)) {
            setState(Failed, QStringLiteral("Could not make the file"));
            return;
        }
        made.close();
        m_undoFrom.append(target);
        m_undoTo.append(QString());
        setState(Done);
        return;
    }

    if (m_kind == NewFolder) {
        const QString name = QFileInfo::exists(QDir(m_destination).filePath(m_sources.value(0)))
            ? freeName(m_destination, m_sources.value(0), true) : m_sources.value(0);
        const QString target = QDir(m_destination).filePath(name);
        if (name.isEmpty() || !QDir().mkdir(target)) {
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
        if (QFileInfo::exists(into)) {
            const QString free = freeName(info.absolutePath(), withoutArchiveEnding(info.fileName()), true);
            if (free.isEmpty()) {
                setState(Failed, QStringLiteral("Could not find a free name to unpack into"));
                return;
            }
            into = QDir(info.absolutePath()).filePath(free);
        }
        if (!QDir().mkpath(into)) {
            setState(Failed, QStringLiteral("Could not make a folder to unpack into"));
            return;
        }
        report(info.fileName(), 1);

        QProcess tar;
        tar.setWorkingDirectory(into);
        { QMutexLocker lock(&m_mutex); m_tar = &tar; }
        tar.start(QStringLiteral("bsdtar"), { QStringLiteral("-xf"), info.absoluteFilePath() });
        const bool unpacked = tar.waitForFinished(-1) && tar.exitCode() == 0;
        { QMutexLocker lock(&m_mutex); m_tar = nullptr; }
        if (!unpacked) {
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
        { QMutexLocker lock(&m_mutex); m_tar = &tar; }
        tar.start(QStringLiteral("bsdtar"), args);
        const bool packed = tar.waitForFinished(-1) && tar.exitCode() == 0;
        { QMutexLocker lock(&m_mutex); m_tar = nullptr; }
        if (!packed) {
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
            const QString free = freeName(info.absolutePath(), info.fileName(), info.isDir());
            const QString target = QDir(info.absolutePath()).filePath(free);
            report(info.fileName(), ++made);
            if (free.isEmpty() || !copyTree(source, target)) {
                setState(Failed, QStringLiteral("Could not duplicate ") + info.fileName());
                return;
            }
            m_undoFrom.append(target);
            m_undoTo.append(QString());
        }
        setState(Done);
        return;
    }

    if (m_kind == Restore || m_kind == PutBack) {
        int back = 0;
        for (int i = 0; i < m_sources.size() && i < m_targets.size(); ++i) {
            if (m_cancelled) {
                setState(Cancelled);
                return;
            }
            const QString from = m_sources.at(i);
            QString to = m_targets.at(i);
            report(QFileInfo(to).fileName(), ++back);
            QDir().mkpath(QFileInfo(to).absolutePath());

            // Something has taken the place it came from since it left. Putting it back over that
            // without asking would lose a file the person never touched.
            if (QFileInfo::exists(to)) {
                const QString name = QFileInfo(to).fileName();
                const QString folder = QFileInfo(to).absolutePath();
                switch (ask(name)) {
                case Skip:
                    continue;
                case Keep: {
                    const QString free = freeName(folder, name, QFileInfo(from).isDir());
                    if (free.isEmpty())
                        continue;
                    to = QDir(folder).filePath(free);
                    break;
                }
                case Replace:
                    m_replaced = true;
                    break;
                }
            }

            if (!moveOne(from, to)) {
                setState(Failed, QStringLiteral("Could not put back ") + QFileInfo(to).fileName());
                return;
            }
            // The trash keeps a note beside what it holds; taking the thing out takes the note too.
            // Only the trash has notes, so only a restore out of it removes one.
            if (m_kind == Restore) {
                const QString files = QFileInfo(from).absolutePath();
                if (files.endsWith(QStringLiteral("/files")))
                    QFile::remove(files.chopped(5) + QStringLiteral("info/")
                                  + QFileInfo(from).fileName() + QStringLiteral(".trashinfo"));
            }
        }
        setState(m_cancelled ? Cancelled : Done);
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
    // Going over something that was already there cannot be taken back: the thing it replaced is
    // gone, and undoing would only delete what took its place.
    if (m_replaced)
        return false;
    // A job that failed or was stopped part way has still done part of it, and that part is exactly
    // what wants undoing.
    return (m_state == Done || m_state == Failed || m_state == Cancelled) && !m_undoFrom.isEmpty()
        && (m_kind == Move || m_kind == Trash || m_kind == Copy || m_kind == Rename
            || m_kind == NewFolder || m_kind == NewFile || m_kind == Extract || m_kind == Compress || m_kind == RenameMany
            || m_kind == Duplicate);
}

FileJobs::FileJobs(QObject *parent) : QObject(parent) {}

QString FileJobs::undoLabel() const {
    switch (m_undo.kind) {
    case FileJob::Move: return QStringLiteral("Undo move");
    case FileJob::Trash: return QStringLiteral("Undo move to trash");
    case FileJob::Copy: return QStringLiteral("Undo copy");
    case FileJob::Rename: return QStringLiteral("Undo rename");
    case FileJob::NewFolder: return QStringLiteral("Undo new folder");
    case FileJob::NewFile: return QStringLiteral("Undo new file");
    case FileJob::Extract: return QStringLiteral("Undo unpack");
    case FileJob::Compress: return QStringLiteral("Undo pack");
    case FileJob::RenameMany: return QStringLiteral("Undo rename");
    case FileJob::Duplicate: return QStringLiteral("Undo duplicate");
    case FileJob::PutBack: return QStringLiteral("Undo");
    default: return QStringLiteral("Undo");
    }
}

FileJob *FileJobs::begin(FileJob *job) {
    m_running.append(job);
    emit runningChanged();
    connect(job, &FileJob::finished, this, [this, job] { retire(job); });
    // A job that stops to ask is waiting on a thread; nothing else tells anyone it is waiting.
    connect(job, &FileJob::stateChanged, this, [this, job] {
        if (job->state() == FileJob::Asking)
            emit jobAsking(job);
    });
    job->start();
    return job;
}

void FileJobs::retire(FileJob *job) {
    m_running.removeAll(job);
    emit runningChanged();

    // Whatever was armed before belongs to a state of the disk that no longer holds, so a job that
    // cannot be undone disarms it rather than leaving it pointing at paths that have moved.
    if (job->undoable())
        m_undo = { int(job->kind()), job->m_undoFrom, job->m_undoTo };
    else
        m_undo = {};
    emit canUndoChanged();
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

QString FileJobs::trashPath() const { return homeTrash() + QStringLiteral("/files"); }

// The note beside a trashed thing says where it came from; that is where it goes back to.
FileJob *FileJobs::restoreFromTrash(const QStringList &paths) {
    QStringList from, to;
    for (const QString &path : paths) {
        const QString files = QFileInfo(path).absolutePath();
        if (!files.endsWith(QStringLiteral("/files")))
            continue;
        const QString root = files.chopped(5);
        QFile file(root + QStringLiteral("info/") + QFileInfo(path).fileName() + QStringLiteral(".trashinfo"));
        if (!file.open(QIODevice::ReadOnly | QIODevice::Text))
            continue;
        QString came;
        while (!file.atEnd()) {
            const QByteArray line = file.readLine();
            if (line.startsWith("Path=")) {
                came = QUrl::fromPercentEncoding(line.mid(5).trimmed());
                break;
            }
        }
        if (came.isEmpty())
            continue;
        // A volume's own trash writes where a thing came from relative to that volume.
        if (!came.startsWith(QLatin1Char('/')))
            came = QStorageInfo(files).rootPath() + QLatin1Char('/') + came;
        from.append(path);
        to.append(came);
    }
    if (from.isEmpty())
        return nullptr;

    auto *job = new FileJob(FileJob::Restore, from, QString(), this);
    job->m_targets = to;
    return begin(job);
}

QStringList FileJobs::trashContents() const {
    QStringList out;
    for (const QString &root : allTrashes()) {
        const QDir files(root + QStringLiteral("/files"));
        for (const QString &name : files.entryList(QDir::AllEntries | QDir::Hidden | QDir::System | QDir::NoDotAndDotDot))
            out.append(files.filePath(name));
    }
    return out;
}

FileJob *FileJobs::emptyTrash() {
    QStringList everything;
    for (const QString &root : allTrashes()) {
        const QDir files(root + QStringLiteral("/files"));
        for (const QString &name : files.entryList(QDir::AllEntries | QDir::Hidden | QDir::System | QDir::NoDotAndDotDot))
            everything.append(files.filePath(name));
        const QDir notes(root + QStringLiteral("/info"));
        for (const QString &name : notes.entryList(QDir::Files | QDir::Hidden | QDir::NoDotAndDotDot))
            everything.append(notes.filePath(name));
    }
    return everything.isEmpty() ? nullptr : begin(new FileJob(FileJob::Delete, everything, QString(), this));
}

FileJob *FileJobs::newFile(const QString &parent, const QString &name) {
    return begin(new FileJob(FileJob::NewFile, { name }, parent, this));
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
    if (undo.kind == FileJob::Copy || undo.kind == FileJob::NewFolder || undo.kind == FileJob::NewFile
        || undo.kind == FileJob::Extract || undo.kind == FileJob::Compress
        || undo.kind == FileJob::Duplicate)
        return begin(new FileJob(FileJob::Delete, undo.from, QString(), this));

    // Everything else goes back where it came from, each thing to its own place. This is not a trash
    // restore, so it leaves the trash's notes alone.
    auto *job = new FileJob(FileJob::PutBack, undo.from, QString(), this);
    job->m_targets = undo.to;
    return begin(job);
}
