#include "thumbnails.h"

#include <QCryptographicHash>
#include <QDir>
#include <QFileInfo>
#include <QFile>
#include <QHash>
#include <QtEndian>
#include <QImage>
#include <QImageReader>
#include <QMimeDatabase>
#include <QProcess>
#include <QtConcurrent/QtConcurrentRun>
#include <QStandardPaths>
#include <QFutureWatcher>
#include <QThread>
#include <QUrl>

namespace {

// The spec's two everyday sizes. Anything larger is drawn from the larger one.
constexpr int NormalSize = 128;
constexpr int LargeSize = 256;

QString cacheRoot() {
    return QStandardPaths::writableLocation(QStandardPaths::GenericCacheLocation) + QStringLiteral("/thumbnails");
}

// The spec names a thumbnail by the md5 of the file's uri, not of its contents.
QString cacheFile(const QString &path, int size) {
    const QByteArray uri = QUrl::fromLocalFile(path).toEncoded();
    const QString hash = QString::fromLatin1(QCryptographicHash::hash(uri, QCryptographicHash::Md5).toHex());
    const QString dir = size > NormalSize ? QStringLiteral("large") : QStringLiteral("normal");
    return cacheRoot() + QLatin1Char('/') + dir + QLatin1Char('/') + hash + QStringLiteral(".png");
}

bool isVideo(const QString &path) {
    QMimeDatabase db;
    return db.mimeTypeForFile(path, QMimeDatabase::MatchExtension).name().startsWith(QLatin1String("video/"));
}

QImage decode(const QString &path, int box) {
    if (isVideo(path)) {
        // One frame a little way in, since the first is often black.
        QProcess ffmpeg;
        ffmpeg.start(QStringLiteral("ffmpeg"),
                     { QStringLiteral("-loglevel"), QStringLiteral("quiet"),
                       QStringLiteral("-ss"), QStringLiteral("3"),
                       QStringLiteral("-i"), path,
                       QStringLiteral("-frames:v"), QStringLiteral("1"),
                       QStringLiteral("-vf"), QStringLiteral("scale=%1:-1").arg(box),
                       QStringLiteral("-f"), QStringLiteral("image2pipe"),
                       QStringLiteral("-vcodec"), QStringLiteral("png"), QStringLiteral("-") });
        if (!ffmpeg.waitForFinished(10000))
            ffmpeg.kill();
        return QImage::fromData(ffmpeg.readAllStandardOutput(), "PNG");
    }

    QImageReader reader(path);
    reader.setAutoTransform(true);
    const QSize full = reader.size();
    if (full.isValid() && (full.width() > box || full.height() > box))
        reader.setScaledSize(full.scaled(box, box, Qt::KeepAspectRatio));
    return reader.read();
}

// The png's tEXt chunks, read from the header rather than by decoding: QImageReader only fills its
// text() in once the image itself has been read, and validity is asked of every row on screen.
QHash<QString, QString> pngText(const QString &file) {
    QHash<QString, QString> out;
    QFile f(file);
    if (!f.open(QIODevice::ReadOnly) || f.read(8) != QByteArray::fromHex("89504e470d0a1a0a"))
        return out;

    while (!f.atEnd()) {
        const QByteArray header = f.read(8);
        if (header.size() < 8)
            break;
        const quint32 length = qFromBigEndian<quint32>(reinterpret_cast<const uchar *>(header.constData()));
        const QByteArray type = header.mid(4, 4);
        // Text lives before the image data; past that there is nothing left to find.
        if (type == "IDAT" || type == "IEND")
            break;
        const QByteArray data = f.read(length);
        f.skip(4);
        if (type == "tEXt") {
            const int nul = data.indexOf('\0');
            if (nul > 0)
                out.insert(QString::fromLatin1(data.left(nul)), QString::fromUtf8(data.mid(nul + 1)));
        }
    }
    return out;
}

// A thumbnail is only the file's if it says so: the spec keeps the uri and the mtime in the png, so a
// file edited since is remade rather than shown stale.
bool valid(const QString &file, const QString &path, qint64 mtime) {
    if (!QFileInfo::exists(file))
        return false;
    const QHash<QString, QString> text = pngText(file);
    if (text.value(QStringLiteral("Thumb::MTime")).toLongLong() != mtime)
        return false;
    return text.value(QStringLiteral("Thumb::URI")) == QString::fromLatin1(QUrl::fromLocalFile(path).toEncoded());
}

void toCache(const QImage &image, const QString &file, const QString &path, const QFileInfo &info) {
    QDir().mkpath(QFileInfo(file).absolutePath());
    QImage stamped = image;
    stamped.setText(QStringLiteral("Thumb::URI"), QString::fromLatin1(QUrl::fromLocalFile(path).toEncoded()));
    stamped.setText(QStringLiteral("Thumb::MTime"), QString::number(info.lastModified().toSecsSinceEpoch()));
    stamped.setText(QStringLiteral("Thumb::Size"), QString::number(info.size()));
    // Written beside and moved into place, so a reader never sees half a file.
    const QString temp = file + QStringLiteral(".part");
    if (stamped.save(temp, "PNG")) {
        QFile::remove(file);
        QFile::rename(temp, file);
    }
}

// Runs off the GUI thread: decode, shrink, write. True when the file is there afterwards.
bool make(const QString &path, const QString &file) {
    const QFileInfo info(path);
    if (!info.isFile())
        return false;
    QImage image = decode(path, NormalSize);
    if (image.isNull())
        return false;
    if (image.width() > NormalSize || image.height() > NormalSize)
        image = image.scaled(NormalSize, NormalSize, Qt::KeepAspectRatio, Qt::SmoothTransformation);
    toCache(image, file, path, info);
    return QFileInfo::exists(file);
}

} // namespace

Thumbnails::Thumbnails(QObject *parent) : QObject(parent) {
    // Thumbnailing is not what the machine is for; a few at a time keeps a folder of photos off the fans.
    m_pool.setMaxThreadCount(qBound(2, QThread::idealThreadCount() / 2, 4));
}

bool Thumbnails::canThumbnail(const QString &path) const {
    QMimeDatabase db;
    const QString name = db.mimeTypeForFile(path, QMimeDatabase::MatchExtension).name();
    if (name.startsWith(QLatin1String("video/")))
        return true;
    if (!name.startsWith(QLatin1String("image/")))
        return false;
    // Only what this Qt can actually read; an exotic image would otherwise show an empty frame.
    return QImageReader::supportedMimeTypes().contains(name.toUtf8());
}

QString Thumbnails::thumbnail(const QString &path, qint64 mtime) {
    const QString file = cacheFile(path, NormalSize);
    if (valid(file, path, mtime))
        return file;
    if (m_failed.value(path, -1) == mtime)
        return {};

    if (!m_running.contains(path)) {
        m_running.insert(path);
        QThreadPool *pool = &m_pool;
        auto *watcher = new QFutureWatcher<QString>(this);
        connect(watcher, &QFutureWatcher<QString>::finished, this, [this, watcher, path] {
            finished(path, watcher->result());
            watcher->deleteLater();
        });
        m_wanted.insert(path, mtime);
        watcher->setFuture(QtConcurrent::run(pool, [path, file] { return make(path, file) ? file : QString(); }));
    }
    return {};
}

void Thumbnails::finished(const QString &path, const QString &file) {
    m_running.remove(path);
    if (file.isEmpty())
        m_failed.insert(path, m_wanted.value(path, -1));
    else
        emit ready(path, file);
    m_wanted.remove(path);
}
