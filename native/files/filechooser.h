#pragma once

#include <QDBusAbstractAdaptor>
#include <QDBusArgument>
#include <QDBusContext>
#include <QDBusMessage>
#include <QDBusObjectPath>
#include <QList>
#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QStringList>
#include <QVariantMap>

// The (type, value) pairs a portal filter is made of: type 0 is a glob, type 1 a mime type.
struct FilterRule {
    uint type = 0;
    QString value;
};
// One entry of the dialog's filter list: a name and the rules that decide what it shows.
struct PortalFilter {
    QString name;
    QList<FilterRule> rules;
};
Q_DECLARE_METATYPE(FilterRule)
Q_DECLARE_METATYPE(PortalFilter)

QDBusArgument &operator<<(QDBusArgument &arg, const FilterRule &rule);
const QDBusArgument &operator>>(const QDBusArgument &arg, FilterRule &rule);
QDBusArgument &operator<<(QDBusArgument &arg, const PortalFilter &filter);
const QDBusArgument &operator>>(const QDBusArgument &arg, PortalFilter &filter);

// One application's ask, held until the surface answers it. The reply is delayed on the bus for as
// long as this lives, so exactly one of accept or reject must be called.
class FileChooserRequest : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("Requests come from the portal.")

    Q_PROPERTY(QString title READ title CONSTANT)
    Q_PROPERTY(QString appId READ appId CONSTANT)
    Q_PROPERTY(QString acceptLabel READ acceptLabel CONSTANT)
    Q_PROPERTY(bool save READ save CONSTANT)
    Q_PROPERTY(bool multiple READ multiple CONSTANT)
    Q_PROPERTY(bool directory READ directory CONSTANT)
    Q_PROPERTY(QString currentName READ currentName CONSTANT)
    Q_PROPERTY(QString folder READ folder CONSTANT)
    // [{ name, patterns: ["*.png", ...] }], the first being what the application asked to start on.
    Q_PROPERTY(QVariantList filters READ filters CONSTANT)

public:
    FileChooserRequest(const QDBusMessage &call, const QString &appId, const QString &title,
                       bool save, const QVariantMap &options, QObject *parent = nullptr);

    // SaveFiles asks for a folder and is answered with one uri per name the application named.
    QStringList saveNames() const { return m_saveNames; }

    QString title() const { return m_title; }
    QString appId() const { return m_appId; }
    QString acceptLabel() const { return m_acceptLabel; }
    bool save() const { return m_save; }
    bool multiple() const { return m_multiple; }
    bool directory() const { return m_directory; }
    QString currentName() const { return m_currentName; }
    QString folder() const { return m_folder; }
    QVariantList filters() const { return m_filters; }

    // The chosen paths, and the index into filters the dialog was showing.
    Q_INVOKABLE void accept(const QStringList &paths, int filterIndex = -1);
    Q_INVOKABLE void reject();

    // Whether a name passes the filter at that index. An index outside the list passes everything.
    Q_INVOKABLE bool matches(const QString &name, int filterIndex) const;

signals:
    void answered();

private:
    void reply(uint response, const QVariantMap &results);

    QDBusMessage m_call;
    QString m_appId;
    QString m_title;
    QString m_acceptLabel;
    bool m_save = false;
    bool m_multiple = false;
    bool m_directory = false;
    QString m_currentName;
    QString m_folder;
    QStringList m_saveNames;
    QVariantList m_filters;
    QList<PortalFilter> m_rules;
    bool m_answered = false;
};

// The shell's own FileChooser backend. The D-Bus slots live on the adaptor below, because a QML type
// cannot carry the reference out-parameters the interface is written with.
class FileChooserPortal : public QObject, protected QDBusContext {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

    Q_PROPERTY(FileChooserRequest *current READ current NOTIFY currentChanged)
    // Whether the bus name is ours. False means applications are still getting somebody else's dialog.
    Q_PROPERTY(bool serving READ serving NOTIFY servingChanged)

public:
    explicit FileChooserPortal(QObject *parent = nullptr);

    FileChooserRequest *current() const { return m_queue.isEmpty() ? nullptr : m_queue.first(); }
    bool serving() const { return m_serving; }

    // Called from the adaptor, inside the D-Bus call, so the reply can be held back.
    uint take(const QString &appId, const QString &title, bool save, const QVariantMap &options);

signals:
    void currentChanged();
    void servingChanged();

private:
    void retire(FileChooserRequest *request);

    QList<FileChooserRequest *> m_queue;
    bool m_serving = false;
};

// The interface as xdg-desktop-portal calls it.
class FileChooserAdaptor : public QDBusAbstractAdaptor {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.freedesktop.impl.portal.FileChooser")
    Q_PROPERTY(uint version READ version CONSTANT)

public:
    explicit FileChooserAdaptor(FileChooserPortal *portal);

    uint version() const { return 3; }

public slots:
    uint OpenFile(const QDBusObjectPath &handle, const QString &appId, const QString &parentWindow,
                  const QString &title, const QVariantMap &options, QVariantMap &results);
    uint SaveFile(const QDBusObjectPath &handle, const QString &appId, const QString &parentWindow,
                  const QString &title, const QVariantMap &options, QVariantMap &results);
    uint SaveFiles(const QDBusObjectPath &handle, const QString &appId, const QString &parentWindow,
                   const QString &title, const QVariantMap &options, QVariantMap &results);

private:
    FileChooserPortal *m_portal;
};
