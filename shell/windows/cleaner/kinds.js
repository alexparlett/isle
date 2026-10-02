.pragma library

// A colour and a glyph for each of DiskUsage.kindNames, index for index. Data, not the accent: D88.
var colors = ["#8E9099", "#FF8A4C", "#F2C94C", "#FF6FA8", "#B48CFF", "#9BD86A", "#D6A77A", "#7FA6FF", "#5AC8FA", "#4FD1C5"];
var glyphs = ["circle-dot", "gamepad-2", "film", "image", "music", "file-text", "archive", "code", "package", "database-backup"];

function color(kind) { return colors[kind] || colors[0]; }
function glyph(kind) { return glyphs[kind] || glyphs[0]; }
