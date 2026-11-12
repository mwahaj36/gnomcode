// Presentation helpers for the Devices list. Display only: no access decisions live here.
const ONLINE_WINDOW_MS = 3 * 60 * 1000;

/** Relative "last seen" text, or null when the timestamp is missing or invalid. */
export function relativeTime(iso, now = Date.now()) {
  const then = Date.parse(iso);
  if (!Number.isFinite(then)) return null;
  const seconds = Math.max(0, Math.round((now - then) / 1000));
  if (seconds < 45) return 'just now';
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  return `${Math.round(hours / 24)} d ago`;
}

/** A desktop counts as online when its heartbeat is recent. */
export function isOnline(lastSeen, now = Date.now()) {
  const then = Date.parse(lastSeen);
  return Number.isFinite(then) && now - then <= ONLINE_WINDOW_MS;
}

/** Phone or desktop browser, from the enrolled device name. */
export function deviceKind(name) {
  return /^phone\b/i.test(name || '') ? 'phone' : 'browser';
}

/** Split enrolled browsers into labelled groups; empty groups are omitted. */
export function groupDevices(requests, mineIds = []) {
  const mine = new Set(mineIds);
  const groups = [
    { key: 'this', label: 'This device', rows: [] },
    { key: 'approved', label: 'Approved', rows: [] },
    { key: 'pending', label: 'Pending', rows: [] },
    { key: 'revoked', label: 'Revoked', rows: [] },
  ];
  for (const row of requests) {
    const key = row.revoked ? 'revoked' : mine.has(row.id) ? 'this' : row.state === 'approved' ? 'approved' : 'pending';
    groups.find(group => group.key === key).rows.push(row);
  }
  return groups.filter(group => group.rows.length);
}
