//! Pending tool approvals answered from the console or a paired phone
//! (`SPEC §3.10`, ADR 0032). Pure state machine: callers inject time and ids.
//!
//! Safety rules enforced here, not in callers:
//! - a request carries only the tool name and a redacted, bounded summary;
//! - a decision is single-use: the first one wins, a different later one is a conflict;
//! - nothing is ever allowed by default: expiry and cancellation resolve to "no decision",
//!   which hands the prompt back to the agent's own dialog.

use crate::redact::redact_summary;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Longest time a hook may hold the agent's prompt waiting for a decision.
pub const HOLD_SECS: u64 = 90;
/// Hook `timeout` written into the agent settings: the hold plus a small margin.
pub const HOOK_TIMEOUT_SECS: u64 = 100;
/// Most approvals that may be pending at once; further requests fall through to the agent.
pub const MAX_PENDING: usize = 16;
/// Seconds a finished approval stays queryable so a repeated tap stays idempotent.
pub const KEEP_DECIDED_SECS: u64 = 600;
/// Longest accepted approval id.
pub const MAX_ID_LEN: usize = 64;

/// What the person chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Let this one tool call run.
    Allow,
    /// Refuse this tool call.
    Deny,
}

impl Decision {
    /// Parses `allow`/`approve` and `deny`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "allow" | "approve" => Some(Self::Allow),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }
}

/// How far an allow reaches. Anything wider than one call is a standing grant for one
/// session, held in memory only: it ends with the session or the daemon, whichever is first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// This one tool call.
    #[default]
    Once,
    /// Every later call of the same tool in this session.
    Tool,
    /// Every later tool call in this session.
    Session,
}

impl Scope {
    /// Parses `once`, `tool` and `session`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "once" => Some(Self::Once),
            "tool" => Some(Self::Tool),
            "session" => Some(Self::Session),
            _ => None,
        }
    }
}

/// Tool label of a request to widen the agent's own sandbox or permissions. A standing grant
/// never answers it (SPEC invariant 7): the person decides each one.
pub const ESCALATION_TOOL: &str = "Permissions";
/// Most standing grants kept at once; the oldest is dropped first.
pub const MAX_GRANTS: usize = 64;
/// Longest a standing grant lasts when nothing reports that its session ended.
pub const GRANT_SECS: u64 = 8 * 60 * 60;

/// A standing allow the person gave for one session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Grant {
    /// Agent slug.
    pub agent: String,
    /// Session the grant is limited to.
    pub session_id: String,
    /// The one tool it covers, or `None` for every tool.
    pub tool: Option<String>,
    /// Unix seconds when it lapses on its own.
    pub expires_at: u64,
}

/// Where a request stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Waiting for a decision.
    Pending,
    /// Allowed once.
    Allowed,
    /// Denied.
    Denied,
    /// No decision arrived in time; the agent's own prompt takes over.
    Expired,
    /// The agent stopped waiting (answered natively, session ended).
    Cancelled,
}

/// Which surface recorded the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decider {
    /// The local console.
    Console,
    /// A paired phone over Remote.
    Phone,
}

/// One approval request as shown to the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Approval {
    /// Random request id; the only handle a decision may use.
    pub id: String,
    /// Agent slug.
    pub agent: String,
    /// Agent-native session id.
    pub session_id: String,
    /// Tool the agent wants to run.
    pub tool: String,
    /// Redacted, bounded description of the call.
    pub summary: String,
    /// Unix seconds when the request arrived.
    pub created_at: u64,
    /// Unix seconds when it stops waiting.
    pub expires_at: u64,
    /// Current state.
    pub state: State,
    /// Who decided, once decided.
    pub decided_by: Option<Decider>,
}

/// Fields of a new approval request.
#[derive(Debug, Clone, Copy)]
pub struct NewApproval<'a> {
    /// Request id (see [`valid_id`]).
    pub id: &'a str,
    /// Agent slug.
    pub agent: &'a str,
    /// Agent-native session id.
    pub session_id: &'a str,
    /// Tool name.
    pub tool: &'a str,
    /// Summary; redacted again on entry.
    pub summary: &'a str,
}

/// Why a request could not be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateError {
    /// Too many pending approvals, or the id is invalid or already used.
    Rejected,
}

/// Why a decision was not applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecideError {
    /// No such request (wrong or unknown id).
    NotFound,
    /// A different decision was already recorded.
    Conflict,
    /// The request expired or was cancelled before an allow arrived.
    Gone,
}

/// How a valid decision was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// This call recorded the decision.
    Recorded,
    /// The same decision was already recorded; nothing changed.
    Repeated,
}

/// In-memory set of approvals. Never persisted.
#[derive(Debug, Default)]
pub struct ApprovalBook {
    items: Vec<Approval>,
    grants: Vec<Grant>,
}

impl ApprovalBook {
    fn sweep(&mut self, now: u64) {
        for item in &mut self.items {
            if item.state == State::Pending && now >= item.expires_at {
                item.state = State::Expired;
            }
        }
        self.items.retain(|item| {
            item.state == State::Pending || now < item.expires_at + KEEP_DECIDED_SECS
        });
        self.grants.retain(|grant| now < grant.expires_at);
    }

    /// Adds a pending request that stops waiting after `hold_secs`.
    ///
    /// # Errors
    /// [`CreateError::Rejected`] when the id is invalid or in use, or too many are pending.
    pub fn create(
        &mut self,
        new: &NewApproval<'_>,
        now: u64,
        hold_secs: u64,
    ) -> Result<Approval, CreateError> {
        self.sweep(now);
        let pending = self
            .items
            .iter()
            .filter(|item| item.state == State::Pending)
            .count();
        if !valid_id(new.id) || pending >= MAX_PENDING || self.items.iter().any(|i| i.id == new.id)
        {
            return Err(CreateError::Rejected);
        }
        let mut item = Approval {
            id: new.id.to_owned(),
            agent: bounded(new.agent, 32),
            session_id: bounded(new.session_id, 128),
            tool: bounded(new.tool, 80),
            // Re-redacted here so no caller can store an unredacted summary.
            summary: redact_summary("", new.summary),
            created_at: now,
            expires_at: now + hold_secs,
            state: State::Pending,
            decided_by: None,
        };
        if self.granted(&item.session_id, &item.tool) {
            // The person already allowed this for the session, at the console.
            item.state = State::Allowed;
            item.decided_by = Some(Decider::Console);
            item.expires_at = now;
        }
        self.items.push(item.clone());
        Ok(item)
    }

    /// Whether a standing grant covers `tool` in `session_id`.
    #[must_use]
    pub fn granted(&self, session_id: &str, tool: &str) -> bool {
        tool != ESCALATION_TOOL
            && self.grants.iter().any(|grant| {
                grant.session_id == session_id
                    && grant.tool.as_deref().is_none_or(|covered| covered == tool)
            })
    }

    /// Turns an allowed request into a standing grant for its session. Returns the grant,
    /// or `None` when the scope is one call, the request is unknown or not allowed, or it
    /// asked to widen the agent's own permissions.
    pub fn grant(&mut self, id: &str, scope: Scope, now: u64) -> Option<Grant> {
        self.sweep(now);
        let item = self
            .items
            .iter()
            .find(|item| item.id == id && item.state == State::Allowed)?;
        if item.tool == ESCALATION_TOOL {
            return None;
        }
        let grant = Grant {
            agent: item.agent.clone(),
            session_id: item.session_id.clone(),
            tool: match scope {
                Scope::Once => return None,
                Scope::Tool => Some(item.tool.clone()),
                Scope::Session => None,
            },
            expires_at: now + GRANT_SECS,
        };
        // A whole-session grant replaces the per-tool ones; a repeat refreshes the old one.
        self.grants.retain(|held| {
            held.session_id != grant.session_id || (grant.tool.is_some() && held.tool != grant.tool)
        });
        if self.grants.len() >= MAX_GRANTS {
            self.grants.remove(0);
        }
        self.grants.push(grant.clone());
        Some(grant)
    }

    /// Ends every standing grant for a session. Returns whether any existed.
    pub fn revoke(&mut self, session_id: &str) -> bool {
        let before = self.grants.len();
        self.grants.retain(|grant| grant.session_id != session_id);
        self.grants.len() != before
    }

    /// Standing grants, oldest first.
    pub fn grants(&mut self, now: u64) -> &[Grant] {
        self.sweep(now);
        &self.grants
    }

    /// Records a decision for the exact request id.
    ///
    /// Deny never fails for a known request unless an allow was already delivered.
    /// Allow needs the request to still be pending.
    ///
    /// # Errors
    /// See [`DecideError`].
    pub fn decide(
        &mut self,
        id: &str,
        decision: Decision,
        by: Decider,
        now: u64,
    ) -> Result<Applied, DecideError> {
        self.sweep(now);
        let item = self
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or(DecideError::NotFound)?;
        let target = match decision {
            Decision::Allow => State::Allowed,
            Decision::Deny => State::Denied,
        };
        match item.state {
            State::Pending => {
                item.state = target;
                item.decided_by = Some(by);
                Ok(Applied::Recorded)
            }
            state if state == target => Ok(Applied::Repeated),
            State::Allowed | State::Denied => Err(DecideError::Conflict),
            State::Expired | State::Cancelled => match decision {
                Decision::Deny => Ok(Applied::Repeated),
                Decision::Allow => Err(DecideError::Gone),
            },
        }
    }

    /// Marks a still-pending request cancelled (the agent stopped waiting).
    pub fn cancel(&mut self, id: &str, now: u64) {
        self.sweep(now);
        if let Some(item) = self
            .items
            .iter_mut()
            .find(|item| item.id == id && item.state == State::Pending)
        {
            item.state = State::Cancelled;
        }
    }

    /// Current state of one request, after expiry is applied.
    pub fn state(&mut self, id: &str, now: u64) -> Option<State> {
        self.sweep(now);
        self.items
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.state)
    }

    /// Requests still waiting, oldest first.
    pub fn pending(&mut self, now: u64) -> Vec<Approval> {
        self.sweep(now);
        self.items
            .iter()
            .filter(|item| item.state == State::Pending)
            .cloned()
            .collect()
    }

    /// Unix seconds when the request stops waiting.
    #[must_use]
    pub fn expires_at(&self, id: &str) -> Option<u64> {
        self.items
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.expires_at)
    }
}

/// An id is 8 to 64 characters of letters, digits, `-` or `_`.
#[must_use]
pub fn valid_id(id: &str) -> bool {
    (8..=MAX_ID_LEN).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn bounded(value: &str, max: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect()
}

/// Redacted one-line description of a tool call for the approval card.
///
/// Reads only the field that identifies the call (command, file name, URL host and path,
/// query, pattern) and never includes file contents or edit bodies. Unknown tools show
/// an empty summary, so the card carries the tool name alone.
#[must_use]
pub fn summarize_tool(tool_name: &str, tool_input: &Value) -> String {
    let text = |key: &str| tool_input.get(key).and_then(Value::as_str).unwrap_or("");
    let detail = match tool_name {
        "Bash" | "BashOutput" | "Monitor" => text("command").to_owned(),
        "Edit" | "MultiEdit" | "Write" | "Read" => short_path(text("file_path")),
        "NotebookEdit" => short_path(text("notebook_path")),
        "WebFetch" => text("url")
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .to_owned(),
        "WebSearch" => text("query").to_owned(),
        "Glob" | "Grep" => text("pattern").to_owned(),
        "Task" | "Agent" => text("description").to_owned(),
        _ => String::new(),
    };
    redact_summary("", &detail)
}

fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect();
    let start = parts.len().saturating_sub(2);
    parts[start..].join("/")
}

/// Whether text appended to the agent transcript shows that the pending prompt was answered
/// at the terminal (the tool ran or was refused, so a tool result was written).
///
/// Verified with Claude Code 2.1.322: the native dialog stays on screen while a
/// `PermissionRequest` hook waits, and Claude does not cancel the hook when the dialog is
/// answered, so the hook watches the transcript and stops waiting on its own.
#[must_use]
pub fn transcript_shows_answer(appended: &str) -> bool {
    appended.contains("\"type\":\"tool_result\"") || appended.contains("\"type\": \"tool_result\"")
}

/// JSON a Claude Code `PermissionRequest` hook prints to resolve the prompt.
///
/// Allow carries no `updatedInput` or `updatedPermissions`, so it covers this one call only
/// and never adds a standing rule; Claude still evaluates its own deny and ask rules.
#[must_use]
pub fn claude_hook_output(decision: Decision) -> String {
    let decision = match decision {
        Decision::Allow => serde_json::json!({"behavior": "allow"}),
        Decision::Deny => serde_json::json!({
            "behavior": "deny",
            "message": "Denied from Gnomcode.",
            "interrupt": false
        }),
    };
    serde_json::json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": decision}})
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn req<'a>(id: &'a str, summary: &'a str) -> NewApproval<'a> {
        NewApproval {
            id,
            agent: "claude-code",
            session_id: "s1",
            tool: "Bash",
            summary,
        }
    }

    fn book_with(now: u64) -> ApprovalBook {
        let mut book = ApprovalBook::default();
        book.create(&req("req-0001", "ls"), now, 90).unwrap();
        book
    }

    #[test]
    fn first_decision_wins_and_repeat_is_idempotent() {
        let mut book = book_with(100);
        assert_eq!(
            book.decide("req-0001", Decision::Allow, Decider::Phone, 101),
            Ok(Applied::Recorded)
        );
        assert_eq!(
            book.decide("req-0001", Decision::Allow, Decider::Console, 102),
            Ok(Applied::Repeated)
        );
        assert_eq!(
            book.decide("req-0001", Decision::Deny, Decider::Console, 103),
            Err(DecideError::Conflict)
        );
        assert_eq!(book.state("req-0001", 104), Some(State::Allowed));
    }

    #[test]
    fn a_session_grant_answers_later_calls_only_in_its_own_session_and_scope() {
        let mut book = book_with(100);
        // Nothing is granted before the request itself is allowed.
        assert_eq!(book.grant("req-0001", Scope::Session, 100), None);
        book.decide("req-0001", Decision::Allow, Decider::Console, 101)
            .unwrap();
        assert_eq!(book.grant("req-0001", Scope::Once, 101), None);
        let grant = book.grant("req-0001", Scope::Tool, 101).unwrap();
        assert_eq!(grant.tool.as_deref(), Some("Bash"));

        let same = book.create(&req("req-0002", "pwd"), 102, 90).unwrap();
        assert_eq!(same.state, State::Allowed);
        let mut other_tool = req("req-0003", "a.rs");
        other_tool.tool = "Edit";
        assert_eq!(
            book.create(&other_tool, 102, 90).unwrap().state,
            State::Pending
        );
        let mut other_session = req("req-0004", "ls");
        other_session.session_id = "s2";
        assert_eq!(
            book.create(&other_session, 102, 90).unwrap().state,
            State::Pending
        );

        // Widening to the whole session covers other tools, but never a permission request.
        book.grant("req-0001", Scope::Session, 103).unwrap();
        assert_eq!(book.grants(103).len(), 1);
        let mut edit = req("req-0005", "b.rs");
        edit.tool = "Edit";
        assert_eq!(book.create(&edit, 104, 90).unwrap().state, State::Allowed);
        let mut escalation = req("req-0006", "network access");
        escalation.tool = ESCALATION_TOOL;
        assert_eq!(
            book.create(&escalation, 104, 90).unwrap().state,
            State::Pending
        );
        book.decide("req-0006", Decision::Allow, Decider::Console, 105)
            .unwrap();
        assert_eq!(book.grant("req-0006", Scope::Session, 105), None);

        // Revoking, or the time limit, puts the person back in charge.
        assert!(book.revoke("s1"));
        assert_eq!(
            book.create(&req("req-0007", "ls"), 106, 90).unwrap().state,
            State::Pending
        );
        book.decide("req-0007", Decision::Allow, Decider::Console, 107)
            .unwrap();
        book.grant("req-0007", Scope::Session, 107).unwrap();
        let later = 107 + GRANT_SECS;
        assert_eq!(
            book.create(&req("req-0008", "ls"), later, 90)
                .unwrap()
                .state,
            State::Pending
        );
    }

    #[test]
    fn wrong_id_is_rejected() {
        let mut book = book_with(100);
        assert_eq!(
            book.decide("req-0002", Decision::Allow, Decider::Phone, 101),
            Err(DecideError::NotFound)
        );
        assert_eq!(book.state("req-0001", 101), Some(State::Pending));
    }

    #[test]
    fn expiry_never_allows() {
        let mut book = book_with(100);
        assert_eq!(book.state("req-0001", 189), Some(State::Pending));
        assert_eq!(book.state("req-0001", 190), Some(State::Expired));
        assert_eq!(
            book.decide("req-0001", Decision::Allow, Decider::Phone, 191),
            Err(DecideError::Gone)
        );
        assert_eq!(
            book.decide("req-0001", Decision::Deny, Decider::Phone, 191),
            Ok(Applied::Repeated)
        );
        assert!(book.pending(191).is_empty());
    }

    #[test]
    fn cancelled_request_cannot_be_allowed() {
        let mut book = book_with(100);
        book.cancel("req-0001", 101);
        assert_eq!(
            book.decide("req-0001", Decision::Allow, Decider::Console, 102),
            Err(DecideError::Gone)
        );
    }

    #[test]
    fn deny_after_allow_is_a_conflict() {
        let mut book = book_with(100);
        book.decide("req-0001", Decision::Allow, Decider::Phone, 101)
            .unwrap();
        assert_eq!(
            book.decide("req-0001", Decision::Deny, Decider::Phone, 102),
            Err(DecideError::Conflict)
        );
    }

    #[test]
    fn pending_is_capped_and_ids_are_unique() {
        let mut book = ApprovalBook::default();
        for n in 0..MAX_PENDING {
            book.create(&req(&format!("req-{n:04}"), "x"), 1, 90)
                .unwrap();
        }
        assert_eq!(
            book.create(&req("req-9999", "x"), 1, 90),
            Err(CreateError::Rejected)
        );
        let mut small = book_with(1);
        assert_eq!(
            small.create(&req("req-0001", "x"), 1, 90),
            Err(CreateError::Rejected)
        );
        assert_eq!(
            small.create(&req("bad id!", "x"), 1, 90),
            Err(CreateError::Rejected)
        );
    }

    #[test]
    fn decided_items_are_dropped_after_retention() {
        let mut book = book_with(100);
        book.decide("req-0001", Decision::Deny, Decider::Phone, 101)
            .unwrap();
        assert!(book.state("req-0001", 190 + KEEP_DECIDED_SECS).is_none());
    }

    #[test]
    fn stored_summary_is_redacted_even_if_the_caller_forgot() {
        let mut book = ApprovalBook::default();
        let item = book
            .create(
                &req(
                    "req-0001",
                    "curl -H 'Authorization: Bearer sk-abcdef1234567890abcdef' API_KEY=hunter2hunter2",
                ),
                1,
                90,
            )
            .unwrap();
        assert!(!item.summary.contains("sk-abcdef"));
        assert!(!item.summary.contains("hunter2"));
    }

    #[test]
    fn summary_reads_only_the_identifying_field() {
        let write = json!({"file_path": "/home/me/proj/src/lib.rs", "content": "SECRET_BODY"});
        assert_eq!(summarize_tool("Write", &write), "src/lib.rs");
        let bash =
            json!({"command": "echo hi && export TOKEN=abcdefghijklmnop1234567890abcdefghijkl"});
        assert!(!summarize_tool("Bash", &bash).contains("abcdefghijklmnop"));
        let fetch = json!({"url": "https://example.com/a?key=sk-verysecretvalue123"});
        assert_eq!(summarize_tool("WebFetch", &fetch), "https://example.com/a");
        assert_eq!(
            summarize_tool("mcp__srv__tool", &json!({"secret": "x"})),
            ""
        );
        let long = json!({"command": "x ".repeat(500)});
        assert!(summarize_tool("Bash", &long).chars().count() <= crate::redact::MAX_SUMMARY_CHARS);
    }

    #[test]
    fn hook_output_matches_claude_schema_and_never_adds_rules() {
        let allow: Value = serde_json::from_str(&claude_hook_output(Decision::Allow)).unwrap();
        let inner = &allow["hookSpecificOutput"];
        assert_eq!(inner["hookEventName"], "PermissionRequest");
        assert_eq!(inner["decision"]["behavior"], "allow");
        assert!(inner["decision"].get("updatedPermissions").is_none());
        assert!(inner["decision"].get("updatedInput").is_none());
        let deny: Value = serde_json::from_str(&claude_hook_output(Decision::Deny)).unwrap();
        assert_eq!(deny["hookSpecificOutput"]["decision"]["behavior"], "deny");
        assert_eq!(deny["hookSpecificOutput"]["decision"]["interrupt"], false);
    }

    #[test]
    fn transcript_answer_is_a_tool_result_not_other_progress() {
        let progress = r#"{"type":"attachment","attachment":{"type":"hook_progress"}}"#;
        assert!(!transcript_shows_answer(progress));
        let result =
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t"}]}}"#;
        assert!(transcript_shows_answer(result));
        assert!(!transcript_shows_answer(""));
    }

    #[test]
    fn decision_parse_accepts_known_words_only() {
        assert_eq!(Decision::parse("Approve"), Some(Decision::Allow));
        assert_eq!(Decision::parse("deny"), Some(Decision::Deny));
        assert_eq!(Decision::parse("always"), None);
    }
}
