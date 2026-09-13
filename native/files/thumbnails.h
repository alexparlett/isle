#pragma once

#include <QHash>
#include <QObject>
#include <QQmlEngine>
#include <QSet>
#include <QString>
#include <QThreadPool>

// Thumbnails as files on disk, in the freedesktop cache, so what other applications made is reused,
// what this makes outlives it, and a view loads one with an ordinary file url.
class Thumbnails : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

public:
    explicit Thumbnails(QObject *parent = nullptr);

    // Whether a file is one a thumbnail can be made of, so a view knows to ask.
    Q_INVOKABLE bool canThumbnail(const QString &path) const;

    // The thumbnail's file if it is already there for a file last written at mtime, and otherwise an
    // empty string and a job started; ready() then names the file. The mtime comes from the row the
    // view is drawing, so a file written since is a different question and asked again on its own.
    Q_INVOKABLE QString thumbnail(const QString &path, qint64 mtime);

signals:
    void ready(const QString &path, const QString &file);

private:
    void finished(const QString &path, const QString &file);

    QThreadPool m_pool;
    // What is already being made, so a view scrolling back over a row does not ask twice.
    QSet<QString> m_running;
};
