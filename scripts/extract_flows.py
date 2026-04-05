"""mitmproxy addon: extract request/response JSON from captured flows.

Usage: mitmdump -r captures/notion-cal.flow -n -s scripts/extract_flows.py
"""
import json
import os
import sys

OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "..", "research", "artifacts", "captures", "json")
os.makedirs(OUTPUT_DIR, exist_ok=True)

REDACT_KEYS = {"accessToken", "refreshToken", "preAuthToken", "cookie", "set-cookie"}
seen = {}  # path -> count


def redact(obj):
    if isinstance(obj, dict):
        return {
            k: "[REDACTED]" if k in REDACT_KEYS and isinstance(v, str) else redact(v)
            for k, v in obj.items()
        }
    if isinstance(obj, list):
        return [redact(v) for v in obj]
    return obj


def response(flow):
    if flow.request.method != "POST":
        return
    if "calendar-api.notion.so" not in flow.request.host:
        return

    path = flow.request.path.split("?")[0]  # e.g. /v2/getUser
    name = path.replace("/v2/", "")

    # Track duplicates
    count = seen.get(name, 0)
    seen[name] = count + 1
    suffix = f"-{count}" if count > 0 else ""

    # Request body
    req_body = None
    try:
        req_body = json.loads(flow.request.get_text())
    except Exception:
        pass

    # Response body
    resp_body = None
    try:
        resp_body = json.loads(flow.response.get_text())
    except Exception:
        pass

    out = {
        "path": path,
        "status": flow.response.status_code if flow.response else None,
        "request": redact(req_body) if req_body else None,
        "response": redact(resp_body) if resp_body else None,
    }

    outfile = os.path.join(OUTPUT_DIR, f"{name}{suffix}.json")
    with open(outfile, "w") as f:
        json.dump(out, f, indent=2, ensure_ascii=False)

    sys.stderr.write(f"  saved {name}{suffix}.json ({flow.response.status_code})\n")
