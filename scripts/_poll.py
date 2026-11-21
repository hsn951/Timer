import json
import time
import urllib.request

UA = {"User-Agent": "macaron-timer"}
RUN_ID = 44918107946
BASE = "https://api.github.com/repos/hsn951/Timer"


def get(url):
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.load(resp)


last = None
for _ in range(60):
    try:
        run = get(f"{BASE}/actions/runs/{RUN_ID}")
    except Exception as exc:  # noqa: BLE001
        print("poll error:", exc, flush=True)
        time.sleep(20)
        continue
    state = f"{run['status']}/{run.get('conclusion')}"
    if state != last:
        print(time.strftime("%H:%M:%S"), state, flush=True)
        last = state
    if run["status"] == "completed":
        jobs = get(f"{BASE}/actions/runs/{RUN_ID}/jobs")
        for job in jobs["jobs"]:
            print("JOB", job["name"], job["conclusion"], flush=True)
            for step in job["steps"]:
                if step.get("conclusion") not in ("success", "skipped", None):
                    print("  STEP_FAILED:", step["name"], step["conclusion"], flush=True)
        break
    time.sleep(20)

try:
    releases = get(f"{BASE}/releases")
    print("RELEASES", len(releases), flush=True)
    for rel in releases:
        print(rel["tag_name"], "draft=", rel["draft"], flush=True)
        for asset in rel["assets"]:
            print("  asset:", asset["name"], round(asset["size"] / 1024 / 1024, 2), "MB", flush=True)
except Exception as exc:  # noqa: BLE001
    print("release error:", exc, flush=True)
