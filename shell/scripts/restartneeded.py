#!/usr/bin/env python3
"""Whether the machine runs older code than it has installed, so that a restart is due: JSON
{restart, reasons}. The kernel, when the running one's module tree is gone (pacman removes it with the
upgrade), and the NVIDIA driver, when the module in the kernel is not the version of nvidia-utils.
"""
import json, os, re, subprocess

reasons = []
running = os.uname().release
if not os.path.isdir("/usr/lib/modules/" + running):
    installed = sorted(d for d in os.listdir("/usr/lib/modules") if os.path.isdir("/usr/lib/modules/" + d + "/kernel")) if os.path.isdir("/usr/lib/modules") else []
    reasons.append("The kernel was updated: %s is running, %s is installed" % (running, installed[-1] if installed else "another"))
try:
    nvrm = open("/proc/driver/nvidia/version").read()
    m = re.search(r"NVRM version:.*?(\d+\.\d+(?:\.\d+)?)", nvrm)
    p = subprocess.run(["pacman", "-Q", "nvidia-utils"], capture_output=True, text=True)
    have = p.stdout.split()[1].split("-")[0] if p.returncode == 0 else ""
    if m and have and m.group(1) != have:
        reasons.append("The NVIDIA driver was updated: %s is loaded, %s is installed; until then nothing can use the GPU through Vulkan" % (m.group(1), have))
except OSError:
    pass
print(json.dumps({"restart": bool(reasons), "reasons": reasons}))
