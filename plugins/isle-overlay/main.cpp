// The badge Dolphin draws on a file of a cloud drive (D65).
//
// A GTK file manager reads the state from the file's own metadata, which anything may write. Dolphin reads
// none of that and asks its overlay plugins instead, once per visible row, so the answer comes from the
// drive's control socket - a query against its index, which takes a fraction of a millisecond - and is held
// briefly so that scrolling a folder does not ask again for what it just asked.

#include <KOverlayIconPlugin>
#include <QDateTime>
#include <QDir>
#include <QHash>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLocalSocket>
#include <QUrl>

namespace
{
// How long an answer stands before the drive is asked again.
constexpr qint64 HeldMs = 2000;
constexpr int WaitMs = 40;

QString drivesRoot()
{
    return QDir::homePath() + QStringLiteral("/Drives/");
}

QString socketFor(const QString &drive)
{
    const QByteArray runtime = qgetenv("XDG_RUNTIME_DIR");
    const QString base = runtime.isEmpty() ? QStringLiteral("/run/user/1000") : QString::fromLocal8Bit(runtime);
    return base + QStringLiteral("/isle/drives/") + drive + QStringLiteral(".sock");
}

// "/home/me/Drives/Proton Drive/a/b" -> ("Proton Drive", "a/b"); false when the path is not on a drive.
bool onADrive(const QUrl &url, QString *drive, QString *within)
{
    if (!url.isLocalFile())
        return false;
    const QString path = url.toLocalFile();
    const QString root = drivesRoot();
    if (!path.startsWith(root))
        return false;
    const QString rest = path.mid(root.size());
    const int cut = rest.indexOf(QLatin1Char('/'));
    if (cut < 0)
        return false;
    *drive = rest.left(cut);
    *within = rest.mid(cut + 1);
    return !drive->isEmpty() && !within->isEmpty();
}

QString askDrive(const QString &drive, const QString &within)
{
    QLocalSocket sock;
    sock.connectToServer(socketFor(drive));
    if (!sock.waitForConnected(WaitMs))
        return QString();
    QJsonObject ask;
    ask.insert(QStringLiteral("op"), QStringLiteral("state"));
    ask.insert(QStringLiteral("path"), within);
    sock.write(QJsonDocument(ask).toJson(QJsonDocument::Compact) + '\n');
    if (!sock.waitForBytesWritten(WaitMs) || !sock.waitForReadyRead(WaitMs))
        return QString();
    const QJsonObject said = QJsonDocument::fromJson(sock.readAll()).object();
    return said.value(QStringLiteral("state")).toString();
}
}

class IsleOverlay : public KOverlayIconPlugin
{
    Q_PLUGIN_METADATA(IID "org.kde.overlayicon.isle")
    Q_OBJECT

public:
    QStringList getOverlays(const QUrl &item) override
    {
        QString drive;
        QString within;
        if (!onADrive(item, &drive, &within))
            return {};

        const QString key = drive + QLatin1Char('\n') + within;
        const qint64 now = QDateTime::currentMSecsSinceEpoch();
        const auto held = m_held.constFind(key);
        if (held != m_held.constEnd() && now - held->when < HeldMs)
            return held->icons;

        QStringList icons;
        const QString state = askDrive(drive, within);
        if (state == QLatin1String("cloud"))
            icons << QStringLiteral("cloud-download");
        else if (state == QLatin1String("cached"))
            icons << QStringLiteral("vcs-normal");
        else if (state == QLatin1String("pinned"))
            icons << QStringLiteral("emblem-favorite");
        else if (state == QLatin1String("uploading"))
            icons << QStringLiteral("view-refresh");
        else if (state == QLatin1String("conflict"))
            icons << QStringLiteral("emblem-important");

        m_held.insert(key, Held{now, icons});
        return icons;
    }

private:
    struct Held {
        qint64 when;
        QStringList icons;
    };
    QHash<QString, Held> m_held;
};

#include "main.moc"
