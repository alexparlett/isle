#!/usr/bin/env python3
"""XKB's registry as JSON: layouts with their variants, and the options by group, with the names people read."""
import json, xml.etree.ElementTree as ET

root = ET.parse("/usr/share/X11/xkb/rules/evdev.xml").getroot()
layouts = []
for l in root.find("layoutList").findall("layout"):
    ci = l.find("configItem")
    vl = l.find("variantList")
    layouts.append({"name": ci.findtext("name"), "description": ci.findtext("description"),
                    "variants": [{"name": v.find("configItem").findtext("name"), "description": v.find("configItem").findtext("description")} for v in (vl.findall("variant") if vl is not None else [])]})
layouts.sort(key=lambda l: l["description"].lower())
groups = []
for g in root.find("optionList").findall("group"):
    ci = g.find("configItem")
    groups.append({"name": ci.findtext("name"), "description": ci.findtext("description"),
                   "options": [{"name": o.find("configItem").findtext("name"), "description": o.find("configItem").findtext("description")} for o in g.findall("option")]})
models = []
for m in root.find("modelList").findall("model"):
    ci = m.find("configItem")
    models.append({"name": ci.findtext("name"), "description": ci.findtext("description"), "vendor": ci.findtext("vendor") or ""})
models.sort(key=lambda m: (m["vendor"] != "Generic", m["vendor"].lower(), m["description"].lower()))
print(json.dumps({"layouts": layouts, "groups": groups, "models": models}))
