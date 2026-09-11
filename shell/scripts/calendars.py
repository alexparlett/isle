#!/usr/bin/env python3
"""Subscribed calendars: fetch every ICS, expand its events over a window around now, print JSON.

    calendars.py '<subscriptions json>' [--cached]

Each subscription is {id, name, url, color}; a url may be a file path, http(s) or webcal. A fetch that fails
falls back to the last copy, kept under ~/.cache/isle/calendars. --cached skips fetching altogether.
"""
import datetime, json, os, sys, urllib.request

CACHE = os.path.join(os.environ.get("XDG_CACHE_HOME", os.path.expanduser("~/.cache")), "isle", "calendars")
BACK, AHEAD = 60, 400  # days around today that are expanded; the widget browses months, the island looks at hours

try:
    import icalendar, recurring_ical_events
    LIBS = None
except ImportError as e:
    LIBS = "needs python-icalendar and python-recurring-ical-events"

subs = json.loads(sys.argv[1]) if len(sys.argv) > 1 else []
cached_only = "--cached" in sys.argv
os.makedirs(CACHE, exist_ok=True)


def fetch(url):
    if url.startswith("webcal://"):
        url = "https://" + url[len("webcal://"):]
    if url.startswith("file://"):
        url = url[len("file://"):]
    if not url.startswith(("http://", "https://")):
        return open(os.path.expanduser(url), "rb").read()
    req = urllib.request.Request(url, headers={"User-Agent": "Isle/1.0 (calendar subscription)"})
    with urllib.request.urlopen(req, timeout=25) as r:
        return r.read()


def local(dt):
    """A local naive datetime, or a date for an all-day event."""
    if isinstance(dt, datetime.datetime):
        if dt.tzinfo is not None:
            dt = dt.astimezone()
        return dt.replace(tzinfo=None, microsecond=0)
    return dt


def expand(data, sub):
    cal = icalendar.Calendar.from_ical(data)
    today = datetime.date.today()
    start, end = today - datetime.timedelta(days=BACK), today + datetime.timedelta(days=AHEAD)
    out = []
    for ev in recurring_ical_events.of(cal).between(start, end):
        s = ev.get("DTSTART")
        if s is None:
            continue
        s = local(s.dt)
        e = ev.get("DTEND")
        e = local(e.dt) if e is not None else None
        all_day = not isinstance(s, datetime.datetime)
        if e is None:
            e = s + datetime.timedelta(days=1) if all_day else s
        out.append({
            "cal": sub["id"], "uid": str(ev.get("UID", "")), "title": str(ev.get("SUMMARY", "") or "(untitled)"),
            "start": s.isoformat(), "end": e.isoformat(), "allDay": all_day,
            "location": str(ev.get("LOCATION", "") or ""),
        })
    return out


calendars, events = [], []
for sub in subs:
    path = os.path.join(CACHE, sub["id"] + ".ics")
    status = {"id": sub["id"], "ok": False, "error": "", "fetched": 0, "count": 0}
    data = None
    if not cached_only:
        try:
            data = fetch(sub.get("url", ""))
            open(path, "wb").write(data)
        except Exception as e:  # the last copy stands in
            status["error"] = str(e).split("\n")[0][:120]
    if data is None and os.path.exists(path):
        data = open(path, "rb").read()
    if data is not None:
        status["fetched"] = int(os.path.getmtime(path)) if os.path.exists(path) else 0
        if LIBS:
            status["error"] = LIBS
        else:
            try:
                evs = expand(data, sub)
                events += evs
                status["ok"] = True
                status["count"] = len(evs)
            except Exception as e:
                status["error"] = "Not a calendar: " + str(e)[:100]
    elif not status["error"]:
        status["error"] = "Nothing fetched yet"
    calendars.append(status)

events.sort(key=lambda e: (e["start"], e["title"]))
print(json.dumps({"calendars": calendars, "events": events}))
