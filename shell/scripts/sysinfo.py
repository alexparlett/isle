#!/usr/bin/env python3
"""One JSON snapshot for the Monitor window: every process with live rates, the CPU split, memory, disk and power
totals, temperatures, fans, GPU, sleep inhibitors and the control daemons.

Rates are deltas against the previous snapshot, kept in $XDG_RUNTIME_DIR/isle-sysinfo.json.
"""
import re
import glob, json, os, pwd, shutil, subprocess, time, urllib.request

def run(args, timeout=1.5):
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=timeout).stdout
    except Exception:
        return ""

STATE = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "isle-sysinfo.json")
CLK = os.sysconf("SC_CLK_TCK")
PAGE = os.sysconf("SC_PAGE_SIZE")
NCPU = os.cpu_count() or 1
now = time.time()
try:
    prev = json.load(open(STATE))
except Exception:
    prev = {"t": 0, "p": {}}
prev.setdefault("p", {})
dt = max(0.001, now - prev["t"])
btime = 0
for line in open("/proc/stat"):
    if line.startswith("btime"):
        btime = int(line.split()[1])
        break
meminfo = {}
for line in open("/proc/meminfo"):
    k, _, v = line.partition(":")
    meminfo[k] = int(v.split()[0]) * 1024
mem_total = meminfo.get("MemTotal", 0)

users = {}
def user(uid):
    if uid not in users:
        try: users[uid] = pwd.getpwuid(uid).pw_name
        except Exception: users[uid] = str(uid)
    return users[uid]

STATES = {"R": "Running", "S": "Sleeping", "D": "Waiting", "Z": "Zombie", "T": "Stopped", "t": "Traced", "I": "Idle"}
procs, cur = [], {}
threads_total = 0
for d in os.listdir("/proc"):
    if not d.isdigit():
        continue
    pid = int(d)
    try:
        stat = open(f"/proc/{pid}/stat").read()
        rp = stat.rindex(")")
        name = stat[stat.index("(") + 1:rp]
        f = stat[rp + 2:].split()
        state, ppid = f[0], int(f[1])
        utime, stime = int(f[11]), int(f[12])
        nice, nthreads = int(f[16]), int(f[17])
        start_ticks = int(f[19])
        rss = int(f[21]) * PAGE
        st = os.stat(f"/proc/{pid}")
        uid = st.st_uid
    except Exception:
        continue
    if state == "Z" and rss == 0 and nthreads == 0:
        pass
    cputicks = utime + stime
    rd = wr = 0
    try:
        for line in open(f"/proc/{pid}/io"):
            if line.startswith("read_bytes"): rd = int(line.split()[1])
            elif line.startswith("write_bytes"): wr = int(line.split()[1])
    except Exception:
        pass
    try:
        cmd = open(f"/proc/{pid}/cmdline", "rb").read().replace(b"\0", b" ").decode(errors="replace").strip()
    except Exception:
        cmd = ""
    p = prev["p"].get(d)
    cpu = ((cputicks - p["c"]) / CLK / dt * 100 / NCPU) if p else 0.0
    ior = (rd - p["r"]) / dt if p else 0.0
    iow = (wr - p["w"]) / dt if p else 0.0
    cur[d] = {"c": cputicks, "r": rd, "w": wr}
    threads_total += nthreads
    procs.append({"pid": pid, "ppid": ppid, "name": name, "user": user(uid), "state": STATES.get(state, state), "threads": nthreads,
                  "cpu": round(max(0.0, cpu), 1), "rss": rss, "memPct": round(rss / mem_total * 100, 1) if mem_total else 0,
                  "ioRead": max(0.0, ior), "ioWrite": max(0.0, iow), "ioReadTotal": rd, "ioWriteTotal": wr, "cmd": cmd[:400] or f"[{name}]",
                  "started": btime + start_ticks // CLK, "cputime": cputicks / CLK, "nice": nice, "kernel": cmd == ""})

# The CPU split, as a delta of the summary line in /proc/stat.
stat_now = [int(x) for x in open("/proc/stat").readline().split()[1:8]]
cpu_split = {"user": 0, "system": 0, "idle": 100}
if prev.get("s"):
    d = [a - b for a, b in zip(stat_now, prev["s"])]
    total = sum(d) or 1
    cpu_split = {"user": round((d[0] + d[1]) * 100 / total, 1), "system": round((d[2] + d[5] + d[6]) * 100 / total, 1), "idle": round((d[3] + d[4]) * 100 / total, 1)}

# Whole-disk throughput from /proc/diskstats.
disk_rd = disk_wr = 0
for line in open("/proc/diskstats"):
    f = line.split()
    if len(f) < 10 or not re.match(r"^(sd[a-z]+|vd[a-z]+|nvme\d+n\d+|mmcblk\d+)$", f[2]):
        continue
    disk_rd += int(f[5]) * 512
    disk_wr += int(f[9]) * 512
disk_rate = {"read": 0.0, "write": 0.0}
if prev.get("d"):
    disk_rate = {"read": max(0.0, (disk_rd - prev["d"][0]) / dt), "write": max(0.0, (disk_wr - prev["d"][1]) / dt)}

# Package power from RAPL when the kernel lets a user read it; battery draw otherwise.
energy = None
for f in sorted(glob.glob("/sys/class/powercap/intel-rapl:*/energy_uj")):
    try:
        energy = int(open(f).read()); break
    except Exception:
        pass
package_w = None
if energy is not None and prev.get("e") is not None and energy >= prev["e"]:
    package_w = round((energy - prev["e"]) / 1e6 / dt, 1)
battery_w = None
for f in glob.glob("/sys/class/power_supply/BAT*/power_now"):
    try:
        battery_w = round(int(open(f).read()) / 1e6, 1); break
    except Exception:
        pass

try:
    json.dump({"t": now, "p": cur, "s": stat_now, "d": [disk_rd, disk_wr], "e": energy}, open(STATE, "w"))
except Exception:
    pass

# What is holding off sleep or idle, with the PID that holds it.
inhibitors = []
for line in run(["systemd-inhibit", "--list", "--no-legend", "--no-pager"]).splitlines():
    f = line.split()
    if len(f) < 6:
        continue
    try:
        inhibitors.append({"who": f[0], "what": f[-5], "why": " ".join(f[1:-5]), "mode": f[-4], "pid": int(f[-2])})
    except Exception:
        pass

load = open("/proc/loadavg").read().split()[:3]
out = {"processes": procs,
       "summary": {"count": len(procs), "threads": threads_total, "load": load, "cpus": NCPU, "cpu": cpu_split,
                   "mem": {"total": mem_total, "available": meminfo.get("MemAvailable", 0), "free": meminfo.get("MemFree", 0),
                           "cached": meminfo.get("Cached", 0) + meminfo.get("SReclaimable", 0), "buffers": meminfo.get("Buffers", 0),
                           "swapTotal": meminfo.get("SwapTotal", 0), "swapUsed": meminfo.get("SwapTotal", 0) - meminfo.get("SwapFree", 0),
                           "dirty": meminfo.get("Dirty", 0)},
                   "disk": disk_rate, "power": {"package": package_w, "battery": battery_w}, "inhibitors": inhibitors},
       "temps": [], "fans": [], "gpu": None, "lact": None, "coolercontrol": None}

# hwmon temperatures with their labels.
for hw in sorted(glob.glob("/sys/class/hwmon/hwmon*")):
    try:
        chip = open(hw + "/name").read().strip()
    except Exception:
        continue
    for t in sorted(glob.glob(hw + "/temp*_input")):
        try:
            val = int(open(t).read()) / 1000
        except Exception:
            continue
        label_file = t.replace("_input", "_label")
        label = open(label_file).read().strip() if os.path.exists(label_file) else os.path.basename(t).replace("_input", "")
        out["temps"].append({"chip": chip, "label": label, "value": val})
    for fan in sorted(glob.glob(hw + "/fan*_input")):
        try:
            rpm = int(open(fan).read())
        except Exception:
            continue
        label_file = fan.replace("_input", "_label")
        label = open(label_file).read().strip() if os.path.exists(label_file) else os.path.basename(fan).replace("_input", "")
        pwm_file = os.path.join(hw, os.path.basename(fan).replace("fan", "pwm").replace("_input", ""))
        pct = None
        try:
            pct = round(int(open(pwm_file).read()) * 100 / 255)
        except Exception:
            pass
        out["fans"].append({"chip": chip, "label": label, "rpm": rpm, "pct": pct})

# GPU: NVIDIA through nvidia-smi, else the first amdgpu card through sysfs.
if shutil.which("nvidia-smi"):
    q = run(["nvidia-smi", "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw,clocks.sm,clocks.mem,fan.speed",
             "--format=csv,noheader,nounits"]).strip()
    if q:
        f = [x.strip() for x in q.split(",")]
        def num(s):
            try: return float(s)
            except Exception: return None
        out["gpu"] = {"vendor": "nvidia", "name": f[0], "util": num(f[1]), "memUsed": (num(f[2]) or 0) * 1e6, "memTotal": (num(f[3]) or 0) * 1e6,
                      "temp": num(f[4]), "power": num(f[5]), "clockCore": num(f[6]), "clockMem": num(f[7]), "fan": num(f[8]), "apps": []}
        # Per-process GPU memory.
        for line in run(["nvidia-smi", "--query-compute-apps=pid,used_memory", "--format=csv,noheader,nounits"]).splitlines():
            try:
                pid, mem = [x.strip() for x in line.split(",")]
                out["gpu"]["apps"].append({"pid": int(pid), "mem": float(mem) * 1e6})
            except Exception:
                pass
else:
    for card in sorted(glob.glob("/sys/class/drm/card?/device")):
        if not os.path.exists(card + "/gpu_busy_percent"):
            continue
        def rd_(p, scale=1):
            try: return float(open(p).read().strip()) / scale
            except Exception: return None
        hw = glob.glob(card + "/hwmon/hwmon*")
        hw = hw[0] if hw else ""
        name = run(["sh", "-c", f"cat {card}/product_name 2>/dev/null || lspci -s $(basename $(readlink {card})) 2>/dev/null | sed 's/.*: //'"]).strip() or "AMD GPU"
        out["gpu"] = {"vendor": "amd", "name": name, "util": rd_(card + "/gpu_busy_percent"), "memUsed": rd_(card + "/mem_info_vram_used"), "memTotal": rd_(card + "/mem_info_vram_total"),
                      "temp": rd_(hw + "/temp1_input", 1000) if hw else None, "power": rd_(hw + "/power1_average", 1e6) if hw else None,
                      "clockCore": None, "clockMem": None, "fan": None, "apps": []}
        break

# LACT: the daemon's socket, if it is running.
if os.path.exists("/run/lactd.sock"):
    try:
        import socket
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); s.settimeout(1)
        s.connect("/run/lactd.sock")
        s.sendall(b'{"command":"list_devices"}\n')
        out["lact"] = {"running": True, "devices": json.loads(s.recv(65536).decode()).get("data", [])}
        s.close()
    except Exception:
        out["lact"] = {"running": False}

# CoolerControl: its REST daemon.
try:
    with urllib.request.urlopen("http://127.0.0.1:11987/health", timeout=0.5) as r:
        out["coolercontrol"] = {"running": r.status == 200}
except Exception:
    out["coolercontrol"] = None

print(json.dumps(out))
