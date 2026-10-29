#!/usr/bin/env node
// Anonymizes a JSONL or JSON agent log file for use in `fixtures/`.
// Preserves structure, token numbers, timestamps, model names, tool names, and rate_limits,
// while scrubbing prompt/response text, tool arguments, file paths, usernames, and secrets.

import fs from "node:fs";
import path from "node:path";

const [, , inputPath, outputPath] = process.argv;
if (!inputPath || !outputPath) {
  console.error("Usage: node anonymize-fixture.mjs <input-file> <output-file>");
  process.exit(1);
}

const PRESERVE_STRING_KEYS = new Set([
  "type",
  "role",
  "status",
  "source",
  "model",
  "model_provider",
  "originator",
  "cli_version",
  "thread_source",
  "stop_reason",
  "service_tier",
  "speed",
  "effort",
  "entrypoint",
  "version",
  "plan_type",
  "limit_id",
  "limit_name",
  "rate_limit_reached_type",
  "created_at",
  "timestamp",
  "name",
  "branch",
]);

const ID_KEYS = new Set([
  "id",
  "uuid",
  "parentUuid",
  "sessionId",
  "session_id",
  "requestId",
  "installation_uuid",
  "creator_user_id",
  "creator_account_id",
  "window_id",
  "commit_hash",
]);

const PATH_KEYS = new Set([
  "cwd",
  "repoRoot",
  "runtime_workspace_roots",
]);

const idMap = new Map();
function scrubId(val) {
  if (!idMap.has(val)) {
    idMap.set(val, `id_${String(idMap.size + 1).padStart(4, "0")}`);
  }
  return idMap.get(val);
}

function scrubUserSettingsChange(text) {
  const matches = [...text.matchAll(/<USER_SETTINGS_CHANGE>[\s\S]*?<\/USER_SETTINGS_CHANGE>/g)];
  if (matches.length === 0) return "[REDACTED_TEXT]";
  return matches.map((m) => m[0]).join("\n");
}

function scrubValue(key, val) {
  if (val === null || typeof val === "number" || typeof val === "boolean") {
    return val;
  }
  if (Array.isArray(val)) {
    if (PATH_KEYS.has(key)) {
      return val.map(() => "/workspace/sample-project");
    }
    return val.map((item) => scrubValue(key, item));
  }
  if (typeof val === "object") {
    const out = {};
    for (const [k, v] of Object.entries(val)) {
      out[k] = scrubValue(k, v);
    }
    return out;
  }
  if (typeof val === "string") {
    if (key === 'text' && val.trimStart().startsWith('<environment_context>')) return '<environment_context>[REDACTED]</environment_context>';
    if (key === 'text' && val.startsWith('# AGENTS.md instructions for ')) return '# AGENTS.md instructions for [REDACTED]\n<INSTRUCTIONS>[REDACTED]</INSTRUCTIONS>';
    if (ID_KEYS.has(key)) return scrubId(val);
    if (PATH_KEYS.has(key)) return "/workspace/sample-project";
    if (key === "gitBranch" || key === "branch") return "main";
    if (key === "repository_url") return "https://github.com/example/sample-project";
    if (PRESERVE_STRING_KEYS.has(key)) return val;
    if (val.includes("<USER_SETTINGS_CHANGE>")) return scrubUserSettingsChange(val);
    return "[REDACTED]";
  }
  return val;
}

const raw = fs.readFileSync(inputPath, "utf8").replace(/^\uFEFF/, "");
const lines = raw.split(/\r?\n/).filter((l) => l.trim().length > 0);
const outLines = lines.map((line, idx) => {
  const cleaned = line.replace(/^\uFEFF/, "").trim();
  const parsed = JSON.parse(cleaned);
  return JSON.stringify(scrubValue("", parsed));
});

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, outLines.join("\n") + "\n", "utf8");
console.log(`Anonymized ${outLines.length} lines -> ${outputPath}`);
