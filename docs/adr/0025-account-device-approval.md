# ADR 0025: Account discovery and device approval without codes

Status: Accepted (2026-11-01)

## Decision

Account-linked desktops register independently in an owner-scoped directory. A signed-in browser requests access using a locally generated, nonexportable X25519 key. An authenticated desktop or an already approved browser approves that specific request with one action. There are no discovery codes, matching codes or phone-side confirmation steps.

The desktop pins the request public key before issuing a reconnect credential over the encrypted channel. Account directory rows and cloud-reported approval state never authorize commands. Each browser/desktop connection has its own room, protected desktop record and browser proof key. Existing challenge-bound directional encryption and command restrictions remain. A compromised account can submit misleading approval requests; users must approve only requests they initiated. Removing independent matching-code verification deliberately trusts the account directory during first enrollment.

Trust lasts until explicit revocation, account disconnect or removal of local browser credentials. Software versions and inactivity do not invalidate it. Desktop identifiers and trust records live in `SPAWNT_HOME`, outside installation directories. Reconnect uses account discovery and bounded reachability checks; a relay connection is not evidence the desktop is online. Failed reconnect never replays a prompt.

The directory stores only owner/device IDs, generic labels, public keys, relay locators, request/revocation state and timestamps. Session lists and conversation content remain on the host and travel only over authenticated E2EE. Each host must be online. Revocation from account Devices is enforced on the next successful desktop directory poll; local revocation closes the channel immediately. Directory failure is visible and does not authorize a new device.

## Consequences

Supersedes ADR 0020's single-desktop, temporary-code discovery and ADR 0021's inactivity expiry for the new flow. Existing remembered connections remain readable for migration, but old enrollment endpoints are removed. Multiple desktops and browsers no longer overwrite each other's trust. Account metadata reporting remains a separate transport, without a separate device-pairing action.
