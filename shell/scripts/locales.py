#!/usr/bin/env python3
"""The generated locales as JSON with the names glibc gives them: [{ code, language, territory }]."""
import json, os, re, subprocess

def field(path, key):
    try:
        for line in open(path, encoding="utf-8", errors="replace"):
            m = re.match(rf'^{key}\s+"(.*)"', line)
            if m:
                return m.group(1)
    except OSError:
        pass
    return ""

out = []
try:
    codes = subprocess.run(["localectl", "list-locales"], capture_output=True, text=True, timeout=10).stdout.split()
except (OSError, subprocess.TimeoutExpired):
    codes = []
for code in codes:
    if code in ("C", "POSIX", "C.UTF-8", "C.utf8"):
        continue
    base = code.split(".")[0].split("@")[0]
    path = f"/usr/share/i18n/locales/{base}"
    # "English" and "United Kingdom" from the address section, not "British English": pickers say English (UK).
    country = field(path, "country_name") or field(path, "territory")
    out.append({"code": code, "language": field(path, "lang_name") or field(path, "language") or base,
                "territory": {"United Kingdom": "UK", "United States": "US"}.get(country, country)})
print(json.dumps(out))
