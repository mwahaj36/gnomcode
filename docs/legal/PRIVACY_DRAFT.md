# Spawnt Privacy Policy — draft for review

This draft is not an approved privacy policy and must not be published. The owner should complete the contact, retention, deletion and service-provider details and obtain legal review before release.

Spawnt is a local-first application. It reads supported agents' local logs to derive session state, usage, cost estimates and quotas. It stores metadata locally. Spawnt-owned conversation history is planned for model switching and session continuity; its contents, retention and deletion controls require a concrete design before release. Agent-owned logs remain under the agents' control. New Resume Queue entries do not store custom prompt text; older installations or backups may contain previously stored text.

Local use does not require an account. Existing opt-in cloud sync can send metadata, usage and quota snapshots to Supabase. It is not end-to-end encrypted. Users choose whether to connect an account. Do not represent this feature as the proposed encrypted Remote service.

Optional update checks, pricing refreshes and any enabled vendor APIs may contact their respective services. The website is hosted on Vercel. Remote and Teams are planned; do not collect waitlist information until the collection mechanism, consent, retention and deletion process are documented.

Before publication, identify the controller and support contact; describe precisely which metadata each enabled service receives, retention and deletion processes, relevant service providers and regions, and applicable user rights. Verify the claims against the released build and its configuration.
