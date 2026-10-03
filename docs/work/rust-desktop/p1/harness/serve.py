#!/usr/bin/env python3
"""Serves P1's boards locally, wrapping each page in the Artifact skeleton."""

import http.server
import io
import os
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[5]

# The same wrapper the Artifact host puts around a published page, so a
# board renders alike in the browser pane and as a published link.
HEAD = b"""<!doctype html><html><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<style>:root{color-scheme:light}body{margin:0;font:14px system-ui,sans-serif}
img{max-width:100%}[hidden]{display:none!important}</style></head><body>
"""
TAIL = b"\n</body></html>\n"


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def send_head(self):
        path = pathlib.Path(self.translate_path(self.path))
        if path.is_dir():
            path = path / "index.html"
        if path.suffix != ".html" or not path.is_file():
            return super().send_head()
        body = path.read_bytes()
        if not body.lstrip().lower().startswith(b"<!doctype"):
            body = HEAD + body + TAIL
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        return io.BytesIO(body)


if __name__ == "__main__":
    port = int(sys.argv[1] if len(sys.argv) > 1 else os.environ.get("PORT", 8766))
    http.server.ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
