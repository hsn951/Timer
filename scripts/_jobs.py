import json, pathlib
data = json.loads(pathlib.Path(r"C:\Users\Admin\AppData\Local\Temp\jobs.json").read_text(encoding="utf-8"))
for job in data.get("jobs", []):
    print("JOB", job["name"], job["status"], job["conclusion"])
    for step in job.get("steps", []):
        print("  ", step["number"], step["name"], "->", step.get("conclusion"))
