#!/usr/bin/env python3
"""Today's sunrise and sunset, minutes after local midnight, for the system timezone.

The coordinates are the timezone's representative city from tzdata's zone1970.tab, so no location service is needed.
Prints {"rise": m, "set": m}; a zone without a city, or polar day or night, falls back to 07:00 and 19:00.
"""
import datetime, json, math, os, sys

FALLBACK = {"rise": 420, "set": 1140}


def coords(tz):
    try:
        for line in open("/usr/share/zoneinfo/zone1970.tab"):
            if line.startswith("#"):
                continue
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 3 or parts[2] != tz:
                continue
            s = parts[1]
            # ±DDMM±DDDMM or ±DDMMSS±DDDMMSS
            i = s.rfind("+") if s.rfind("+") > 0 else s.rfind("-")
            la, lo = s[:i], s[i:]

            def dms(v, deg_len):
                sign = -1 if v[0] == "-" else 1
                v = v[1:]
                d, m = int(v[:deg_len]), int(v[deg_len:deg_len + 2])
                sec = int(v[deg_len + 2:deg_len + 4] or 0)
                return sign * (d + m / 60 + sec / 3600)
            return dms(la, 2), dms(lo, 3)
    except OSError:
        pass
    return None


def sun(lat, lon, day, utc_offset_min):
    # NOAA's approximation; good to a minute or two, which is all a theme needs.
    n = day.timetuple().tm_yday
    lng_hour = lon / 15
    out = []
    for rising in (True, False):
        t = n + ((6 if rising else 18) - lng_hour) / 24
        m = 0.9856 * t - 3.289
        l = (m + 1.916 * math.sin(math.radians(m)) + 0.020 * math.sin(math.radians(2 * m)) + 282.634) % 360
        ra = math.degrees(math.atan(0.91764 * math.tan(math.radians(l)))) % 360
        ra = (ra + (math.floor(l / 90) * 90 - math.floor(ra / 90) * 90)) / 15
        sin_dec = 0.39782 * math.sin(math.radians(l))
        cos_dec = math.cos(math.asin(sin_dec))
        cos_h = (math.cos(math.radians(90.833)) - sin_dec * math.sin(math.radians(lat))) / (cos_dec * math.cos(math.radians(lat)))
        if cos_h > 1 or cos_h < -1:
            return None
        h = (360 - math.degrees(math.acos(cos_h))) if rising else math.degrees(math.acos(cos_h))
        h /= 15
        local = h + ra - 0.06571 * t - 6.622
        ut = (local - lng_hour) % 24
        out.append(round((ut * 60 + utc_offset_min) % 1440))
    return {"rise": out[0], "set": out[1]}


def today():
    tz = os.environ.get("TZ")
    if not tz:
        try:
            tz = os.path.relpath(os.path.realpath("/etc/localtime"), "/usr/share/zoneinfo")
        except OSError:
            tz = None
    c = coords(tz) if tz else None
    if not c:
        return FALLBACK
    now = datetime.datetime.now().astimezone()
    return sun(c[0], c[1], now.date(), int(now.utcoffset().total_seconds() // 60)) or FALLBACK


if __name__ == "__main__":
    print(json.dumps(today()))
