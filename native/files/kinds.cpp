#include "kinds.h"

#include <cctype>
#include <cstring>
#include <unordered_map>

namespace kinds {

namespace {

const std::unordered_map<std::string, int> &folders() {
    static const std::unordered_map<std::string, int> map {
        { "steamapps", Games }, { "Steam", Games }, { ".steam", Games }, { "Games", Games },
        { "lutris", Games }, { "heroic", Games }, { "Heroic", Games }, { "itch", Games },
        { ".wine", Games }, { "compatibilitytools.d", Games }, { "compatdata", Games }, { "pfx", Games },
        { "drive_c", Games },

        { ".cache", Caches }, { "cache", Caches }, { "Cache", Caches }, { "caches", Caches },
        { "Caches", Caches }, { "CachedData", Caches }, { "GPUCache", Caches }, { "Code Cache", Caches },
        { "ShaderCache", Caches }, { "shadercache", Caches }, { "DawnCache", Caches },
        { "GrShaderCache", Caches }, { "thumbnails", Caches }, { "log", Caches }, { "logs", Caches },
        { "Logs", Caches }, { "journal", Caches }, { "coredump", Caches }, { "Crash Reports", Caches },

        { "node_modules", Developer }, { ".git", Developer }, { ".cargo", Developer },
        { ".rustup", Developer }, { ".npm", Developer }, { ".pnpm-store", Developer },
        { ".yarn", Developer }, { ".gradle", Developer }, { ".m2", Developer }, { ".venv", Developer },
        { "venv", Developer }, { "__pycache__", Developer }, { ".nuget", Developer },
        { ".dotnet", Developer }, { ".android", Developer }, { "Android", Developer },
        { "docker", Developer }, { "containers", Developer }, { "target", Developer }, { "site-packages", Developer },
        { "dist-packages", Developer },

        { "flatpak", System }, { "Applications", System }, { "pacman", System },
    };
    return map;
}

// Folders that are the system only where the root has them.
const std::unordered_map<std::string, int> &rootFolders() {
    static const std::unordered_map<std::string, int> map {
        { "usr", System }, { "opt", System }, { "boot", System }, { "etc", System },
    };
    return map;
}

const std::unordered_map<std::string, int> &endings() {
    static const std::unordered_map<std::string, int> map = [] {
        std::unordered_map<std::string, int> m;
        const auto add = [&](int kind, std::initializer_list<const char *> list) {
            for (const char *e : list)
                m.emplace(e, kind);
        };
        add(Video, { "mp4", "mkv", "mov", "avi", "webm", "m4v", "wmv", "flv", "mpg", "mpeg", "ts", "m2ts" });
        add(Images, { "jpg", "jpeg", "png", "gif", "webp", "heic", "heif", "avif", "cr2", "cr3", "nef", "arw",
                      "dng", "raf", "orf", "tif", "tiff", "bmp", "svg", "psd", "xcf", "kra", "exr" });
        add(Audio, { "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma", "aiff", "aif", "alac" });
        add(Documents, { "pdf", "doc", "docx", "odt", "rtf", "txt", "md", "epub", "mobi", "xls", "xlsx",
                         "ods", "ppt", "pptx", "odp", "csv", "tex", "pages", "key", "numbers" });
        add(Archives, { "zip", "tar", "gz", "tgz", "xz", "zst", "bz2", "7z", "rar", "iso", "img", "qcow2",
                        "vdi", "vmdk", "vhd", "vhdx", "dmg" });
        add(Developer, { "c", "h", "cc", "cpp", "hpp", "rs", "go", "py", "js", "mjs", "tsx", "jsx", "java",
                         "kt", "swift", "rb", "php", "cs", "o", "a", "obj", "rlib", "rmeta", "class", "jar",
                         "pyc", "wasm", "pack", "idx", "safetensors", "gguf", "onnx", "pt", "ckpt" });
        add(Games, { "pak", "ucas", "utoc", "vpk", "bsa", "ba2", "gcf", "forge", "wad" });
        add(System, { "so", "exe", "dll", "appimage", "flatpakref", "ko", "efi" });
        add(Caches, { "log" });
        return m;
    }();
    return map;
}

} // namespace

int ofFolder(const char *name, bool atRoot, int inherited) {
    if (atRoot) {
        if (const auto it = rootFolders().find(name); it != rootFolders().end())
            return it->second;
    }
    const auto it = folders().find(name);
    return it != folders().end() ? it->second : inherited;
}

int ofFile(const char *name) {
    const char *dot = std::strrchr(name, '.');
    if (!dot || dot == name || !dot[1])
        return Other;
    std::string ending(dot + 1);
    if (ending.size() > 12)
        return Other;
    for (char &c : ending)
        c = char(std::tolower(static_cast<unsigned char>(c)));
    const auto it = endings().find(ending);
    return it != endings().end() ? it->second : Other;
}

int ofPath(const std::string &path) {
    int kind = None;
    size_t start = 1;
    bool atRoot = true;
    while (start < path.size()) {
        size_t end = path.find('/', start);
        if (end == std::string::npos)
            end = path.size();
        if (end > start)
            kind = ofFolder(path.substr(start, end - start).c_str(), atRoot, kind);
        atRoot = false;
        start = end + 1;
    }
    return kind;
}

QStringList names() {
    return { QStringLiteral("Other"), QStringLiteral("Games"), QStringLiteral("Video"), QStringLiteral("Images"),
             QStringLiteral("Audio"), QStringLiteral("Documents"), QStringLiteral("Archives and disk images"),
             QStringLiteral("Developer"), QStringLiteral("Apps and system"), QStringLiteral("Caches and logs") };
}

} // namespace kinds
