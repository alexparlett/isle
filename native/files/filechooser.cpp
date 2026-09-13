#include "filechooser.h"

#include <QDBusConnection>
#include <QDBusMetaType>
#include <QDir>
#include <QFile>
#include <QMimeDatabase>
#include <QFileInfo>
#include <QRegularExpression>
#include <QUrl>

QDBusArgument &operator<<(QDBusArgument &arg, const FilterRule &rule) {
    arg.beginStructure();
    arg << rule.type << rule.value;
    arg.endStructure();
    return arg;
}

const QDBusArgument &operator>>(const QDBusArgument &arg, FilterRule &rule) {
    arg.beginStructure();
    arg >> rule.type >> rule.value;
    arg.endStructure();
    return arg;
}

QDBusArgument &operator<<(QDBusArgument &arg, const PortalFilter &filter) {
    arg.beginStructure();
    arg << filter.name << filter.rules;
    arg.endStructure();
    return arg;
}

const QDBusArgument &operator>>(const QDBusArgument &arg, PortalFilter &filter) {
    arg.beginStructure();
    arg >> filter.name >> filter.rules;
    arg.endStructure();
    return arg;
}

namespace {

// current_folder and current_file come over the bus as NUL-terminated byte arrays.
QString pathFromBytes(const QVariant &value) {
    QByteArray bytes = value.toByteArray();
    while (bytes.endsWith('\0'))
        bytes.chop(1);
    return bytes.isEmpty() ? QString() : QFile::decodeName(bytes);
}

QVariant unwrap(const QVariantMap &options, const QString &key) {
    const QVariant value = options.value(key);
    return value.canConvert<QDBusVariant>() ? value.value<QDBusVariant>().variant() : value;
}

} // namespace

FileChooserRequest::FileChooserRequest(const QDBusMessage &call, const QString &appId, const QString &title,
                                       bool save, const QVariantMap &options, QObject *parent)
    : QObject(parent), m_call(call), m_appId(appId), m_title(title), m_save(save) {

    m_acceptLabel = unwrap(options, QStringLiteral("accept_label")).toString();
    m_multiple = unwrap(options, QStringLiteral("multiple")).toBool();
    m_directory = unwrap(options, QStringLiteral("directory")).toBool();
    m_currentName = unwrap(options, QStringLiteral("current_name")).toString();

    m_folder = pathFromBytes(unwrap(options, QStringLiteral("current_folder")));
    const QString file = pathFromBytes(unwrap(options, QStringLiteral("current_file")));
    if (!file.isEmpty()) {
        const QFileInfo info(file);
        if (m_folder.isEmpty())
            m_folder = info.absolutePath();
        if (m_currentName.isEmpty())
            m_currentName = info.fileName();
    }
    if (m_folder.isEmpty() || !QFileInfo(m_folder).isDir())
        m_folder = QDir::homePath();

    // SaveFiles names what it wants written; the person only chooses where.
    const QVariant names = unwrap(options, QStringLiteral("files"));
    if (names.isValid()) {
        for (const QVariant &one : names.toList()) {
            const QString name = pathFromBytes(one);
            if (!name.isEmpty())
                m_saveNames.append(name);
        }
        if (!m_saveNames.isEmpty())
            m_directory = true;
    }

    const QVariant raw = unwrap(options, QStringLiteral("filters"));
    if (raw.canConvert<QDBusArgument>())
        raw.value<QDBusArgument>() >> m_rules;

    // The filter the application asked to start on leads the list, so the dialog opens on it.
    PortalFilter chosen;
    const QVariant rawCurrent = unwrap(options, QStringLiteral("current_filter"));
    if (rawCurrent.canConvert<QDBusArgument>()) {
        rawCurrent.value<QDBusArgument>() >> chosen;
        if (!chosen.name.isEmpty()) {
            const int at = [&] {
                for (int i = 0; i < m_rules.size(); ++i)
                    if (m_rules.at(i).name == chosen.name) return i;
                return -1;
            }();
            if (at > 0) m_rules.move(at, 0);
            else if (at < 0) m_rules.prepend(chosen);
        }
    }

    for (const PortalFilter &filter : std::as_const(m_rules)) {
        QStringList patterns;
        QStringList endings;
        for (const FilterRule &rule : filter.rules) {
            patterns.append(rule.value);
            // "*.tar.gz" is ".tar.gz". A rule that is not a glob on an ending has none to show.
            if (rule.type == 0 && rule.value.startsWith(QLatin1String("*."))) {
                const QString ending = rule.value.mid(1).toLower();
                if (!ending.contains(QLatin1Char('*')) && !endings.contains(ending))
                    endings.append(ending);
            }
        }
        // The ending is what a person is choosing between; the application's prose is the fallback
        // for a filter that has no ending to name, "All files" among them.
        QString label = filter.name;
        if (!endings.isEmpty()) {
            label = endings.size() > 3 ? endings.mid(0, 3).join(QStringLiteral(", ")) + QStringLiteral(" …")
                                       : endings.join(QStringLiteral(", "));
        }
        m_filters.append(QVariantMap { { QStringLiteral("name"), filter.name },
                                       { QStringLiteral("label"), label },
                                       { QStringLiteral("patterns"), patterns } });
    }
}

bool FileChooserRequest::matches(const QString &name, int filterIndex) const {
    if (filterIndex < 0 || filterIndex >= m_rules.size())
        return true;

    for (const FilterRule &rule : m_rules.at(filterIndex).rules) {
        // A glob is matched against the name; a mime type against what the name says the file is.
        if (rule.type == 0) {
            const auto re = QRegularExpression::fromWildcard(rule.value, Qt::CaseInsensitive,
                                                             QRegularExpression::UnanchoredWildcardConversion);
            if (re.match(name).hasMatch())
                return true;
        } else {
            QMimeDatabase db;
            const QMimeType type = db.mimeTypeForFile(name, QMimeDatabase::MatchExtension);
            if (type.inherits(rule.value))
                return true;
        }
    }
    return false;
}

void FileChooserRequest::accept(const QStringList &paths, int filterIndex) {
    if (m_answered)
        return;

    QStringList uris;
    if (!m_saveNames.isEmpty() && paths.size() == 1) {
        const QDir folder(paths.first());
        for (const QString &name : m_saveNames)
            uris.append(QUrl::fromLocalFile(folder.filePath(name)).toString());
    } else {
        for (const QString &path : paths)
            uris.append(QUrl::fromLocalFile(path).toString());
    }

    QVariantMap results { { QStringLiteral("uris"), uris }, { QStringLiteral("writable"), true } };
    if (filterIndex >= 0 && filterIndex < m_rules.size())
        results.insert(QStringLiteral("current_filter"), QVariant::fromValue(m_rules.at(filterIndex)));

    reply(0, results);
}

void FileChooserRequest::reject() {
    if (m_answered)
        return;
    reply(1, {});
}

void FileChooserRequest::reply(uint response, const QVariantMap &results) {
    m_answered = true;
    QDBusMessage message = m_call.createReply(QVariantList { response, results });
    QDBusConnection::sessionBus().send(message);
    emit answered();
}

FileChooserPortal::FileChooserPortal(QObject *parent) : QObject(parent) {
    qDBusRegisterMetaType<FilterRule>();
    qDBusRegisterMetaType<QList<FilterRule>>();
    qDBusRegisterMetaType<PortalFilter>();
    qDBusRegisterMetaType<QList<PortalFilter>>();

    new FileChooserAdaptor(this);

    QDBusConnection bus = QDBusConnection::sessionBus();
    // The object path every portal backend answers on; the name is what portals.conf points at.
    const bool object = bus.registerObject(QStringLiteral("/org/freedesktop/portal/desktop"), this);
    m_serving = object && bus.registerService(QStringLiteral("org.freedesktop.impl.portal.desktop.isle"));
}

uint FileChooserPortal::take(const QString &appId, const QString &title, bool save, const QVariantMap &options) {
    // The surface answers whenever the person does, so the call waits rather than the shell blocking.
    setDelayedReply(true);

    auto *request = new FileChooserRequest(message(), appId, title, save, options, this);
    connect(request, &FileChooserRequest::answered, this, [this, request] { retire(request); });
    m_queue.append(request);
    if (m_queue.size() == 1)
        emit currentChanged();
    return 0;
}

void FileChooserPortal::retire(FileChooserRequest *request) {
    const bool wasCurrent = !m_queue.isEmpty() && m_queue.first() == request;
    m_queue.removeAll(request);
    request->deleteLater();
    if (wasCurrent)
        emit currentChanged();
}

FileChooserAdaptor::FileChooserAdaptor(FileChooserPortal *portal)
    : QDBusAbstractAdaptor(portal), m_portal(portal) {
    setAutoRelaySignals(false);
}

uint FileChooserAdaptor::OpenFile(const QDBusObjectPath &, const QString &appId, const QString &,
                                  const QString &title, const QVariantMap &options, QVariantMap &) {
    return m_portal->take(appId, title, false, options);
}

uint FileChooserAdaptor::SaveFile(const QDBusObjectPath &, const QString &appId, const QString &,
                                  const QString &title, const QVariantMap &options, QVariantMap &) {
    return m_portal->take(appId, title, true, options);
}

uint FileChooserAdaptor::SaveFiles(const QDBusObjectPath &, const QString &appId, const QString &,
                                   const QString &title, const QVariantMap &options, QVariantMap &) {
    return m_portal->take(appId, title, true, options);
}
