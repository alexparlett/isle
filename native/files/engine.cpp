#include "engine.h"

#include <QFileInfo>
#include <QMimeDatabase>

Engine::Engine(QObject *parent) : QObject(parent) {}

QString Engine::version() const { return QStringLiteral(ISLE_FILES_VERSION); }

QString Engine::iconNameFor(const QString &path) const {
    QFileInfo info(path);
    if (info.isDir())
        return QStringLiteral("folder");

    QMimeDatabase db;
    const QMimeType type = db.mimeTypeForFile(info, QMimeDatabase::MatchExtension);
    const QString name = type.iconName();
    return name.isEmpty() ? QStringLiteral("text-x-generic") : name;
}
