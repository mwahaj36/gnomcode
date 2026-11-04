export function approvedRequest(requests, waiting) {
  return requests.find(row => row.desktop_id === waiting.desktop && row.id === waiting.request && !row.revoked && row.state === 'approved' && row.offer);
}
