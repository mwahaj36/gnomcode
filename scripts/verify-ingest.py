"""Independent check: raw agent logs vs a Gnomcode database. Read-only on both.

Usage: python scripts/verify-ingest.py <state.db>
  1. Ingest into a scratch DB:  gnomcode --db <tmp>/state.db status
  2. Run this script against it. Every token class should print OK.
Codex archives (*.jsonl.zst) need `pip install zstandard`; without it they are skipped.
Set CLAUDE_ROOT to compare against a frozen copy of a live Claude config directory.
"""
import glob, json, os, sqlite3, sys
from collections import defaultdict

try:
    import zstandard
except ImportError:
    zstandard = None

HOME = os.path.expanduser("~")
db = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)


def lines(path):
    data = open(path, "rb").read()
    if path.endswith(".zst"):
        if zstandard is None:
            return
        data = zstandard.ZstdDecompressor().stream_reader(data).read()
    for raw in data.splitlines():
        try:
            yield json.loads(raw)
        except Exception:
            continue


def db_totals(agent):
    rows = db.execute(
        "select u.session_id, sum(input_tokens), sum(output_tokens), sum(cache_read_tokens),"
        " sum(cache_write_5m_tokens+cache_write_1h_tokens), sum(reasoning_tokens)"
        " from usage_record u join session s on s.id=u.session_id where s.agent=? group by u.session_id",
        (agent,),
    )
    return {r[0]: r[1:] for r in rows}


def report(name, truth, stored, fields):
    missing = [s for s in truth if s not in stored and any(truth[s])]
    extra = [s for s in stored if s not in truth]
    diffs = []
    for sid, t in truth.items():
        d = stored.get(sid)
        if d is None:
            continue
        for i, f in enumerate(fields):
            if (t[i] or 0) != (d[i] or 0):
                diffs.append((sid, f, t[i], d[i]))
    tsum = [sum((v[i] or 0) for v in truth.values()) for i in range(len(fields))]
    dsum = [sum((v[i] or 0) for v in stored.values()) for i in range(len(fields))]
    print(f"\n== {name}: {len(truth)} sessions with usage in logs, {len(stored)} in DB")
    for i, f in enumerate(fields):
        print(f"   {f:12} logs={tsum[i]:>14,}  db={dsum[i]:>14,}  {'OK' if tsum[i]==dsum[i] else 'MISMATCH'}")
    print(f"   missing in DB: {len(missing)}  extra in DB: {len(extra)}  per-session diffs: {len(diffs)}")
    for x in diffs[:5]:
        print("   diff", x)
    for x in missing[:3]:
        print("   missing", x)


# Claude Code: one usage per assistant message id (streamed lines repeat it).
truth = defaultdict(lambda: [0, 0, 0, 0, 0])
for path in glob.glob(os.environ.get("CLAUDE_ROOT", f"{HOME}/.claude") + "/projects/**/*.jsonl", recursive=True):
    seen = set()
    for o in lines(path):
        m = o.get("message") or {}
        u = m.get("usage")
        if o.get("type") != "assistant" or not u:
            continue
        key = (m.get("id"), o.get("requestId"))
        if key in seen:
            continue
        seen.add(key)
        sid = "claude-code:" + (("agent-" + o["agentId"]) if o.get("isSidechain") and o.get("agentId") else o.get("sessionId", ""))
        t = truth[sid]
        t[0] += u.get("input_tokens", 0)
        t[1] += u.get("output_tokens", 0)
        t[2] += u.get("cache_read_input_tokens", 0)
        t[3] += u.get("cache_creation_input_tokens", 0)
report("Claude Code", dict(truth), db_totals("claude-code"), ["input", "output", "cache_read", "cache_write", "reasoning"])

# Codex: final cumulative total per session is ground truth.
truth = {}
for path in glob.glob(f"{HOME}/.codex/sessions/**/rollout-*", recursive=True) + glob.glob(f"{HOME}/.codex/archived_sessions/rollout-*"):
    sid, last = None, None
    for o in lines(path):
        p = o.get("payload") or {}
        if o.get("type") == "session_meta":
            sid = sid or p.get("id")
        if o.get("type") == "event_msg" and p.get("type") == "token_count" and (p.get("info") or {}).get("total_token_usage"):
            last = p["info"]["total_token_usage"]
    if sid and last:
        cached = last.get("cached_input_tokens", 0)
        truth["codex:" + sid] = (last.get("input_tokens", 0), last.get("output_tokens", 0), cached, 0, last.get("reasoning_output_tokens", 0))
report("Codex", truth, db_totals("codex"), ["input", "output", "cache_read", "cache_write", "reasoning"])

# Antigravity: one usage per model call in conversations/<id>.db (gen_metadata), counted once per
# conversation id even when the 2.0 app holds a migrated copy; transcripts only where no .db exists.
def pb_varint(buf, i):
    value = shift = 0
    while True:
        byte = buf[i]
        i += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if not byte & 0x80:
            return value, i


def pb_fields(buf):
    i = 0
    try:
        while i < len(buf):
            key, i = pb_varint(buf, i)
            wire = key & 7
            if wire == 0:
                val, i = pb_varint(buf, i)
            elif wire == 2:
                n, i = pb_varint(buf, i)
                val = buf[i:i + n]
                i += n
            elif wire in (1, 5):
                i += 8 if wire == 1 else 4
                val = 0
            else:
                return
            yield key >> 3, val
    except IndexError:
        return


def pb_get(buf, field):
    return next((v for f, v in pb_fields(buf) if f == field), None)


GEMINI = os.environ.get("GEMINI_ROOT", f"{HOME}/.gemini")
truth = defaultdict(lambda: [0, 0, 0, 0, 0])
calls = {}
for root in ("antigravity-cli", "antigravity-ide", "antigravity"):
    for path in glob.glob(f"{GEMINI}/{root}/conversations/*.db"):
        conv = os.path.basename(path)[:-3]
        con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
        for idx, blob in con.execute("select idx, data from gen_metadata where data is not null"):
            usage = pb_get(pb_get(blob, 1) or b"", 4) or b""
            counts = [pb_get(usage, f) or 0 for f in (2, 3, 5)]
            if any(counts):
                calls.setdefault(conv, {})[idx] = counts
        con.close()
for conv, rows in calls.items():
    t = truth["antigravity:" + conv]
    for counts in rows.values():
        t[0] += counts[0]
        t[1] += counts[1]
        t[2] += counts[2]
seen_steps = {}
for root in ("antigravity", "antigravity-ide", "antigravity-cli"):
    for path in glob.glob(f"{GEMINI}/{root}/brain/*/.system_generated/logs/transcript.jsonl"):
        conv = path.replace("\\", "/").split("/brain/")[1].split("/")[0]
        if conv in calls:
            continue
        steps = seen_steps.setdefault(conv, set())
        for o in lines(path):
            if o.get("type") != "PLANNER_RESPONSE" or o.get("step_index") in steps:
                continue
            steps.add(o.get("step_index"))
            t = truth["antigravity:" + conv]
            t[0] += o.get("input_tokens", 0)
            t[1] += o.get("output_tokens", 0)
            t[2] += o.get("cache_read_tokens", 0)
report("Antigravity", {k: v for k, v in truth.items() if any(v)}, db_totals("antigravity"), ["input", "output", "cache_read", "cache_write", "reasoning"])

print("\n== Models (DB, top per agent)")
for row in db.execute("select agent, model, count(*) from session group by agent, model order by agent, 3 desc"):
    print("  ", row)
