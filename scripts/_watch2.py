import json
import subprocess
import time

UA = "User-Agent: macaron-timer"
BASE = "https://api.github.com/repos/hsn951/Timer"


def get(path):
    out = subprocess.run(
        ["curl.exe", "-s", "-H", UA, BASE + path],
        capture_output=True,
        check=True,
    ).stdout
    return json.loads(out.decode("utf-8"))


run = None
for _ in range(60):
    runs = get("/actions/runs?per_page=1")["workflow_runs"]
    run = runs[0]
    print(time.strftime("%H:%M:%S"), run["id"], run["head_sha"][:7], run["status"], run.get("conclusion"), flush=True)
    if run["status"] == "completed":
        jobs = get(f"/actions/runs/{run['id']}/jobs")
        for job in jobs["jobs"]:
            print("JOB", job["name"], job["conclusion"], flush=True)
            for step in job["steps"]:
                if step.get("conclusion") not in ("success", "skipped", None):
                    print("  FAILED:", step["name"], step["conclusion"], flush=True)
        break
    time.sleep(30)

try:
    rel = get("/releases")
    print("RELEASES", len(rel), flush=True)
    for r in rel:
        print(r["tag_name"], "draft=", r["draft"], r["published_at"], flush=True)
        for a in r["assets"]:
            print("  asset:", a["name"], round(a["size"] / 1024 / 1024, 2), "MB", flush=True)
except Exception as exc:  # noqa: BLE001
    print("release err", exc, flush=True)
