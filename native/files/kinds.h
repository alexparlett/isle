#pragma once

#include <QStringList>

#include <string>

// What a file is, so the disk can be told apart by it (D87).
namespace kinds {

enum Kind : int { Other, Games, Video, Images, Audio, Documents, Archives, Developer, System, Caches, Count };

// No folder above has said what everything under it is.
constexpr int None = -1;

// The kind a folder gives everything under it, or `inherited` when its name says nothing. A folder
// named for a kind wins over the folders above it, so a game's cache is a cache.
int ofFolder(const char *name, bool atRoot, int inherited);
// The kind of a file by its ending, for when no folder has said.
int ofFile(const char *name);
// What ofFolder says for the last component of a path, walking it from the root as a scan does.
int ofPath(const std::string &path);

QStringList names();

} // namespace kinds
