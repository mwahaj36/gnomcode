//! Domain types, pricing, quota, rules and redaction. Pure: no I/O, no async.
//!
//! See `docs/SPEC.md` §3.6–§3.9 for the data model and invariants.

pub mod branding;
pub mod license;
pub mod presence;
pub mod pricing;
pub mod redact;
pub mod remote_crypto;
pub mod rules;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// How much a reported number can be trusted (SPEC §3.1, invariant 6).
///
/// Every cost, quota or token figure surfaced to the user carries one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    /// Read directly from agent data (e.g. token counts in a log line).
    Exact,
    /// Derived by Spawnt heuristics (e.g. Claude 5-hour window estimate).
    Estimated,
    /// Obtained from an opt-in, possibly unofficial vendor API.
    Api,
    /// Not available for this agent.
    Unknown,
}

impl TrustLevel {
    /// Stable lowercase identifier used in storage and the API.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Estimated => "estimated",
            Self::Api => "api",
            Self::Unknown => "unknown",
        }
    }
}

/// Supported AI coding agent surfaces (SPEC §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentId {
    /// Anthropic Claude Code (CLI & VS Code extension).
    ClaudeCode,
    /// OpenAI Codex (CLI & ChatGPT/Codex Desktop App).
    Codex,
    /// Google Antigravity (App, IDE & CLI).
    Antigravity,
    /// Google Gemini CLI.
    GeminiCli,
    /// Cline / Roo Code VS Code extension.
    Cline,
    /// Cursor editor (experimental, opt-in).
    Cursor,
    /// Aider terminal agent (opt-in).
    Aider,
}

impl AgentId {
    /// Stable kebab-case identifier used in database keys and API responses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::Antigravity => "antigravity",
            Self::GeminiCli => "gemini-cli",
            Self::Cline => "cline",
            Self::Cursor => "cursor",
            Self::Aider => "aider",
        }
    }
}

/// Host environment where an agent session runs (SPEC §3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    /// Terminal / command-line session.
    Cli,
    /// Editor extension (VS Code, Cursor, Windsurf, Antigravity IDE).
    Ide,
    /// Standalone desktop app (Antigravity App, ChatGPT / Codex Desktop).
    DesktopApp,
}

impl Surface {
    /// Stable identifier used in SQLite storage and API responses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Ide => "ide",
            Self::DesktopApp => "desktop_app",
        }
    }
}

/// Live state of an agent session (SPEC §3.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Streaming or generating a model response.
    Thinking,
    /// Executing a tool call.
    Tool,
    /// Blocked waiting for user permission or input.
    AwaitingApproval,
    /// Turn completed; ready for next user prompt.
    Idle,
    /// Encountered an API or runtime error.
    Error,
    /// Explicitly reported session closure or completion.
    Ended,
}

impl SessionState {
    /// Stable identifier used in SQLite storage and API responses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Thinking => "thinking",
            Self::Tool => "tool",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Idle => "idle",
            Self::Error => "error",
            Self::Ended => "ended",
        }
    }
}

/// Logical project grouping sessions across clones and git worktrees (SPEC §3.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Deterministic SHA-256 hex prefix (16 chars) of `canonical_key`.
    pub id: String,
    /// Normalized Git remote (`host/owner/repo`) or normalized folder path.
    pub canonical_key: String,
    /// Most recently observed working directory path on disk.
    pub repo_root: String,
    /// Short human-readable project name (e.g. `spawnt`).
    pub name: String,
    /// Last active Git branch, if known.
    pub git_branch: Option<String>,
}

impl Project {
    /// Construct a [`Project`] by normalizing the Git remote URL first (so clones and
    /// worktrees of the same repository share one project ID), falling back to the
    /// normalized `repo_root` directory path when no remote is available.
    #[must_use]
    pub fn from_origin(
        git_remote: Option<&str>,
        repo_root: &str,
        git_branch: Option<&str>,
    ) -> Self {
        let canonical_key = git_remote
            .map(normalize_git_remote)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| normalize_path_key(repo_root));

        let name = canonical_key
            .rsplit('/')
            .find(|seg| !seg.is_empty())
            .unwrap_or("unknown")
            .to_owned();

        let id = short_sha256_hex(&canonical_key);

        Self {
            id,
            canonical_key,
            repo_root: repo_root.to_owned(),
            name,
            git_branch: git_branch.map(str::to_owned),
        }
    }
}

fn normalize_git_remote(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/').trim_end_matches(".git");
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .or_else(|| trimmed.strip_prefix("ssh://"))
        .unwrap_or(trimmed);

    let without_user = without_scheme
        .strip_prefix("git@")
        .unwrap_or(without_scheme);

    // Convert SCP-style `github.com:owner/repo` into `github.com/owner/repo`.
    let normalized = without_user.replacen(':', "/", 1);
    normalized
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_ascii_lowercase()
}

fn normalize_path_key(raw: &str) -> String {
    let replaced = raw.trim().replace('\\', "/");
    let trimmed = replaced.trim_end_matches('/');
    trimmed.to_ascii_lowercase()
}

fn short_sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut out = String::with_capacity(16);
    for byte in &digest[..8] {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Normalized agent session (SPEC §3.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Composite key `"<agent>:<native_id>"`.
    pub id: String,
    /// Agent type.
    pub agent: AgentId,
    /// Host surface (`cli`, `ide`, `desktop_app`).
    pub surface: Surface,
    /// Associated [`Project::id`].
    pub project_id: String,
    /// Parent session ID when this session is a spawned subagent / sidechain.
    pub parent_session_id: Option<String>,
    /// Most recently active model in this session, if known.
    pub model: Option<String>,
    /// Human-readable session title or first task label, if known.
    pub title: Option<String>,
    /// Current state in the session state machine.
    pub state: SessionState,
    /// Live OS process ID matched by the process scanner, if active.
    pub pid: Option<u32>,
    /// Timestamp of the first event in the session.
    pub started_at: DateTime<Utc>,
    /// Timestamp of the most recent event in the session.
    pub last_event_at: DateTime<Utc>,
}

impl Session {
    /// Build the canonical composite session ID `"<agent>:<native_id>"`.
    #[must_use]
    pub fn compose_id(agent: AgentId, native_id: &str) -> String {
        format!("{}:{}", agent.as_str(), native_id)
    }
}

/// Single turn or model invocation token & cost record (SPEC §3.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRecord {
    /// Idempotent deduplication key (`message.id + requestId`, `session + ordinal`, etc.).
    pub dedup_key: String,
    /// Owning [`Session::id`].
    pub session_id: String,
    /// UTC timestamp of the turn.
    pub timestamp: DateTime<Utc>,
    /// Model identifier (`claude-opus-5`, `gpt-6.1-sol`, `Gemini 4 Argon (Medium)`, etc.).
    pub model: String,
    /// Uncached input tokens billed at standard input rate.
    pub input_tokens: u64,
    /// Output tokens (including reasoning tokens where applicable).
    pub output_tokens: u64,
    /// Reasoning/thinking output tokens subset.
    pub reasoning_tokens: u64,
    /// Cached input tokens read at discounted cache-read rate.
    pub cache_read_tokens: u64,
    /// Prompt cache creation tokens with 5-minute TTL.
    pub cache_write_5m_tokens: u64,
    /// Prompt cache creation tokens with 1-hour TTL.
    pub cache_write_1h_tokens: u64,
    /// Total context size (`input + cache_read + cache_write`) of this turn.
    pub context_tokens: u64,
    /// Model maximum context window, if known.
    pub context_window: Option<u64>,
    /// Calculated API-equivalent cost in USD.
    pub cost_usd: f64,
    /// USD saved by prompt cache reads compared to uncached input pricing.
    pub cache_savings_usd: f64,
    /// Trust level of `cost_usd`.
    pub cost_basis: TrustLevel,
    /// Version tag of the pricing table used.
    pub pricing_version: String,
}

impl UsageRecord {
    /// Percentage (`0.0..=100.0+`) of the model's context window filled on this turn.
    #[must_use]
    pub fn context_fill_pct(&self) -> Option<f64> {
        let window = self.context_window?;
        if window == 0 {
            return None;
        }
        Some((self.context_tokens as f64 / window as f64) * 100.0)
    }

    /// Returns `true` when [`Self::context_fill_pct`] meets or exceeds `threshold_pct`.
    #[must_use]
    pub fn is_context_bloated(&self, threshold_pct: f64) -> bool {
        self.context_fill_pct()
            .is_some_and(|pct| pct >= threshold_pct)
    }
}

/// Kind of lifecycle event in the live activity feed (SPEC §3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    /// User prompt or new turn started.
    Prompt,
    /// Tool execution started.
    ToolStart,
    /// Tool execution finished.
    ToolEnd,
    /// Session blocked waiting for user approval.
    ApprovalWait,
    /// API or tool error occurred.
    Error,
    /// Spawned a subagent / sidechain session.
    Subagent,
    /// Session hit a provider/model quota rate limit.
    RateLimited,
    /// Session automatically resumed by the Resume Queue after quota reset.
    AutoResume,
}

/// Redacted activity event stored in the 30-day activity feed (SPEC §3.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// Owning [`Session::id`].
    pub session_id: String,
    /// UTC timestamp of the event.
    pub timestamp: DateTime<Utc>,
    /// Lifecycle event category.
    pub kind: ActivityKind,
    /// Tool name (e.g. `run_command`, `edit_file`, `Bash`), if applicable.
    pub tool_name: Option<String>,
    /// Redacted one-line summary (secrets stripped, truncated to <=120 chars).
    pub summary_redacted: Option<String>,
    /// Execution duration in milliseconds, if known.
    pub duration_ms: Option<u64>,
}

/// Quota window duration bucket (SPEC §3.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaWindow {
    /// 5-hour rolling or fixed window.
    FiveHour,
    /// 7-day weekly window.
    Weekly,
    /// Monthly billing cycle window.
    Monthly,
}

/// Point-in-time snapshot of a per-model quota pool `(provider, limit_id, window)` (SPEC §3.9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    /// Provider slug (`codex`, `antigravity`, `claude-code`, `gemini-cli`, `cursor`).
    pub provider: String,
    /// Model family or pool identifier (`codex`, `premium`, `claude-opus`, `gemini`, `fable`, etc.).
    pub limit_id: String,
    /// Quota window type.
    pub window: QuotaWindow,
    /// Percentage of the quota pool consumed (`0.0..=100.0`).
    pub used_pct: f64,
    /// UTC timestamp when this pool resets, if known.
    pub resets_at: Option<DateTime<Utc>>,
    /// Subscription plan tier (`plus`, `pro`, `max`, etc.), if known.
    pub plan: Option<String>,
    /// Trust level of `used_pct` and `resets_at`.
    pub basis: TrustLevel,
    /// UTC timestamp when this snapshot was observed.
    pub observed_at: DateTime<Utc>,
}

/// Configured subscription plan entry used to compute Plan Value ROI (SPEC §3.16).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    /// Provider or agent slug (e.g. `claude-code`, `codex`, `antigravity`).
    pub provider: String,
    /// Display label (e.g. `Claude Max`, `ChatGPT Plus`).
    pub plan_name: String,
    /// Monthly subscription price in USD.
    pub monthly_price_usd: f64,
}

/// User-configurable retention, backfill, plan, and notification settings (SPEC §3.16).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSettings {
    /// Days to retain raw `usage_record` rows before rolling into `usage_daily` (`None` = forever).
    pub raw_turns_retention_days: Option<u32>,
    /// Days to retain `activity_event` rows (`None` = forever).
    pub activity_retention_days: Option<u32>,
    /// Days of historical logs to backfill on initial scan (`0..=30`).
    pub initial_backfill_days: u32,
    /// Context window fill percentage that triggers a bloat warning (default `75.0`).
    pub context_bloat_threshold_pct: f64,
    /// Seconds without a tool result before flagging `awaiting_approval` (default `4`).
    pub approval_wait_delay_secs: u32,
    /// Active user subscriptions for Plan Value ROI calculation.
    pub subscriptions: Vec<SubscriptionPlan>,
    /// Notify when an agent is awaiting approval.
    pub notify_awaiting_approval: bool,
    /// Notify when a quota pool hits 80% / 95% / 100%.
    pub notify_quota_warning: bool,
    /// Notify when a rate-limited quota pool resets.
    pub notify_quota_reset: bool,
    /// Notify when context window fill crosses `context_bloat_threshold_pct`.
    pub notify_context_bloat: bool,
    /// Notify when a session crosses a spend or turn guardrail.
    pub notify_guardrail_breach: bool,
    /// Play sound with desktop notifications.
    pub notification_sound: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            raw_turns_retention_days: Some(180),
            activity_retention_days: Some(30),
            initial_backfill_days: 30,
            context_bloat_threshold_pct: 75.0,
            approval_wait_delay_secs: 4,
            subscriptions: Vec::new(),
            notify_awaiting_approval: true,
            notify_quota_warning: true,
            notify_quota_reset: true,
            notify_context_bloat: true,
            notify_guardrail_breach: true,
            notification_sound: true,
        }
    }
}

/// Role of a message in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    /// Human user prompt.
    User,
    /// Agent/assistant textual or reasoning response.
    Assistant,
    /// Tool invocation or execution result.
    Tool,
    /// System or lifecycle notification.
    System,
}

impl ChatRole {
    /// Stable string identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
            Self::System => "system",
        }
    }

    /// Parse from stored string representation.
    #[must_use]
    pub fn parse_role(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl std::str::FromStr for ChatRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "tool" => Ok(Self::Tool),
            "system" => Ok(Self::System),
            _ => Err(()),
        }
    }
}

/// A local, redacted structured chat event in a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatEvent {
    /// Unique identifier for the event.
    pub event_id: String,
    /// Session identifier.
    pub session_id: String,
    /// Sequential turn index within the session.
    pub turn_index: i64,
    /// Message role (user, assistant, tool, system).
    pub role: ChatRole,
    /// Redacted textual content of the message or tool summary.
    pub content: String,
    /// Optional tool name for tool-role events.
    pub tool_name: Option<String>,
    /// Optional execution or approval status (e.g. "approved", "denied", "pending", "success", "error").
    pub status: Option<String>,
    /// Creation timestamp (UTC).
    pub created_at: DateTime<Utc>,
}

/// Routing policy for model failover and session continuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoutingPolicy {
    /// Prompt user before initiating any model switch or resume.
    #[default]
    AskFirst,
    /// Automatically switch to the highest-quota available model.
    Automatic,
    /// Prohibit switching to a different AI vendor/provider.
    DenyCrossProvider,
}

impl RoutingPolicy {
    /// Returns whether an automatic switch is allowed between the known providers.
    #[must_use]
    pub fn allows_automatic(
        self,
        source_provider: Option<&str>,
        target_provider: Option<&str>,
    ) -> bool {
        match self {
            Self::AskFirst => false,
            Self::Automatic => true,
            Self::DenyCrossProvider => {
                matches!((source_provider, target_provider), (Some(source), Some(target)) if !source.is_empty() && source.eq_ignore_ascii_case(target))
            }
        }
    }
    /// Stable string identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AskFirst => "ask_first",
            Self::Automatic => "automatic",
            Self::DenyCrossProvider => "deny_cross_provider",
        }
    }

    /// Parse from string representation.
    #[must_use]
    pub fn parse_policy(s: &str) -> Option<Self> {
        match s {
            "ask_first" => Some(Self::AskFirst),
            "automatic" => Some(Self::Automatic),
            "deny_cross_provider" => Some(Self::DenyCrossProvider),
            _ => None,
        }
    }
}

/// Zero-token handoff compilation payload for model or agent switches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffPacket {
    /// Originating session identifier.
    pub session_id: String,
    /// Originating agent surface or model.
    pub source_model: String,
    /// Target model selected for failover or continuation.
    pub target_model: String,
    /// Synthesized, redacted goal prompt and task context.
    pub prompt_summary: String,
    /// List of file paths modified or touched during the session.
    pub touched_files: Vec<String>,
    /// Number of turns summarized.
    pub turn_count: u32,
    /// UTC timestamp of packet compilation.
    pub created_at: DateTime<Utc>,
}

/// Compiles user objectives and tool states without including tool output.
#[must_use]
pub fn handoff_summary(events: &[ChatEvent]) -> String {
    let mut lines = Vec::new();
    for event in events {
        match event.role {
            ChatRole::User => {
                let prompt = redact::clean_user_text(&event.content);
                if !prompt.trim().is_empty() {
                    lines.push(format!("Goal: {}", redact::redact_chat_content(prompt)));
                }
            }
            ChatRole::Tool => {
                if let Some(status) = &event.status {
                    let name = event.tool_name.as_deref().unwrap_or("tool");
                    lines.push(format!(
                        "Tool {}: {}",
                        redact::redact_chat_content(name),
                        redact::redact_chat_content(status)
                    ));
                }
            }
            _ => {}
        }
    }
    lines.join("\n")
}

/// A remote pairing session state for mobile connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemotePairingSession {
    /// Ephemeral room identifier.
    pub room_id: String,
    /// Desktop ephemeral public key (hex-encoded).
    pub public_key: String,
    /// 6-digit confirmation code shown on desktop and phone.
    pub verify_code: String,
    /// Expiration timestamp (UTC).
    pub expires_at: DateTime<Utc>,
}

impl RemotePairingSession {
    /// Computes a 6-digit verification code from shared key material.
    #[must_use]
    pub fn derive_verify_code(shared_secret: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"spawnt-remote-verify-v1");
        hasher.update(shared_secret);
        let result = hasher.finalize();
        let num = u32::from_be_bytes([result[0], result[1], result[2], result[3]]) % 1_000_000;
        format!("{num:06}")
    }
}

/// A sealed ciphertext frame transmitted across the zero-knowledge relay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedRemoteFrame {
    /// Monotonically increasing sequence number to defeat replay attacks.
    pub sequence_num: u32,
    /// 12-byte initialization vector / nonce (hex-encoded).
    pub nonce: String,
    /// AEAD ciphertext (hex-encoded).
    pub ciphertext: String,
}

#[cfg(test)]
mod tests {
    #[test]
    fn remote_cipher_round_trip_rejects_replay_and_tampering() {
        use crate::remote_crypto::{PeerRole, RemoteChannel, RemoteKeyPair};
        let desktop = RemoteKeyPair::from_private([7; 32]);
        let phone = RemoteKeyPair::from_private([9; 32]);
        let mut a =
            RemoteChannel::new(&desktop, phone.public_key(), "test-room", PeerRole::Desktop)
                .expect("desktop channel");
        let mut b = RemoteChannel::new(&phone, desktop.public_key(), "test-room", PeerRole::Phone)
            .expect("phone channel");
        assert_eq!(a.verify_code(), b.verify_code());
        assert_eq!(
            desktop.public_key_hex(),
            "13be4feaeaf204c7fd3358fc9c00721881d174278128227ec674f37f7fe97b6d"
        );
        assert_eq!(
            phone.public_key_hex(),
            "57db4b359f23ae5e146e4e2512056704722506348c150c14753d0c933d04d421"
        );
        assert_eq!(a.verify_code(), "107539");
        let mut vector =
            RemoteChannel::new(&desktop, phone.public_key(), "test-room", PeerRole::Desktop)
                .expect("vector channel");
        let vector_frame = vector
            .seal(br#"{"type":"sessions"}"#)
            .expect("vector frame");
        assert_eq!(
            vector_frame.ciphertext,
            "14ec0cfbb605d66599953653b73328b932fc4b0832f36df8dd6a690a3322ab4ddd82fc"
        );
        let frame = a.seal(b"conversation").expect("seal");
        assert_eq!(b.open(&frame).expect("open"), b"conversation");
        assert!(b.open(&frame).is_err());
        let mut changed = a.seal(b"second").expect("seal second");
        changed.ciphertext.push('0');
        assert!(b.open(&changed).is_err());
        let mut wrong_room =
            RemoteChannel::new(&phone, desktop.public_key(), "other-room", PeerRole::Phone)
                .expect("wrong room channel");
        assert_ne!(a.verify_code(), wrong_room.verify_code());
        assert!(wrong_room.open(&frame).is_err());
        assert!(RemoteChannel::new(&desktop, [0; 32], "test-room", PeerRole::Desktop).is_err());
    }
    use super::{
        AgentId, ChatEvent, ChatRole, EncryptedRemoteFrame, HandoffPacket, Project,
        RemotePairingSession, RoutingPolicy, Session, TrustLevel, UsageRecord, UserSettings,
    };

    #[test]
    fn handoff_keeps_user_goals_and_tool_status_without_tool_output() {
        let now = chrono::Utc::now();
        let event = |role, content: &str, status: Option<&str>| ChatEvent {
            event_id: "event".into(),
            session_id: "codex:1".into(),
            turn_index: 1,
            role,
            content: content.into(),
            tool_name: Some("test".into()),
            status: status.map(str::to_owned),
            created_at: now,
        };
        let events = [
            event(
                ChatRole::User,
                "<environment_context>injected setup</environment_context>",
                None,
            ),
            event(ChatRole::User, "Fix tests", None),
            event(ChatRole::Tool, "private output", Some("failed")),
        ];
        let summary = super::handoff_summary(&events);
        assert!(summary.contains("Fix tests"));
        assert!(summary.contains("failed"));
        assert!(!summary.contains("private output"));
        assert!(!summary.contains("injected setup"));
    }

    #[test]
    fn trust_level_identifiers_are_stable() {
        assert_eq!(TrustLevel::Exact.as_str(), "exact");
        assert_eq!(TrustLevel::Estimated.as_str(), "estimated");
        assert_eq!(TrustLevel::Api.as_str(), "api");
        assert_eq!(TrustLevel::Unknown.as_str(), "unknown");
    }

    #[test]
    fn project_from_git_remote_merges_ssh_and_https_clones() {
        let ssh = Project::from_origin(
            Some("git@github.com:mwahaj36/Spawnt.git"),
            "C:\\Users\\dev\\worktree-1",
            Some("feat/a"),
        );
        let https = Project::from_origin(
            Some("https://github.com/mwahaj36/spawnt/"),
            "/Users/dev/code/spawnt",
            Some("main"),
        );
        assert_eq!(ssh.id, https.id);
        assert_eq!(ssh.canonical_key, "github.com/mwahaj36/spawnt");
        assert_eq!(ssh.name, "spawnt");
    }

    #[test]
    fn project_falls_back_to_normalized_path_when_no_remote() {
        let p1 = Project::from_origin(None, "C:\\Work\\MyApp\\", None);
        let p2 = Project::from_origin(None, "c:/work/myapp", None);
        assert_eq!(p1.id, p2.id);
        assert_eq!(p1.name, "myapp");
    }

    #[test]
    fn session_id_prefixes_agent_slug() {
        let sid = Session::compose_id(AgentId::ClaudeCode, "abc-123");
        assert_eq!(sid, "claude-code:abc-123");
    }

    #[test]
    fn usage_record_context_fill_pct_handles_known_and_zero_windows() {
        let rec = UsageRecord {
            dedup_key: "msg1:req1".to_owned(),
            session_id: "claude-code:s1".to_owned(),
            timestamp: chrono::Utc::now(),
            model: "claude-opus-5".to_owned(),
            input_tokens: 10_000,
            output_tokens: 500,
            reasoning_tokens: 0,
            cache_read_tokens: 140_000,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 150_000,
            context_window: Some(200_000),
            cost_usd: 0.42,
            cache_savings_usd: 1.20,
            cost_basis: TrustLevel::Exact,
            pricing_version: "2026-10-26".to_owned(),
        };
        assert_eq!(rec.context_fill_pct(), Some(75.0));
        assert!(rec.is_context_bloated(75.0));
        assert!(!rec.is_context_bloated(80.0));
    }

    #[test]
    fn user_settings_defaults_match_spec() {
        let s = UserSettings::default();
        assert_eq!(s.raw_turns_retention_days, Some(180));
        assert_eq!(s.activity_retention_days, Some(30));
        assert_eq!(s.initial_backfill_days, 30);
        assert_eq!(s.context_bloat_threshold_pct, 75.0);
        assert!(s.notify_awaiting_approval);
        assert!(s.notify_quota_warning);
        assert!(s.notify_quota_reset);
        assert!(s.notify_context_bloat);
        assert!(s.notify_guardrail_breach);
    }

    #[test]
    fn chat_role_string_round_trip() {
        for role in [
            ChatRole::User,
            ChatRole::Assistant,
            ChatRole::Tool,
            ChatRole::System,
        ] {
            assert_eq!(ChatRole::parse_role(role.as_str()), Some(role));
        }
        assert_eq!(ChatRole::parse_role("invalid"), None);
    }

    #[test]
    fn chat_event_serializes_and_deserializes() {
        let event = ChatEvent {
            event_id: "evt-123".to_owned(),
            session_id: "claude-code:s1".to_owned(),
            turn_index: 1,
            role: ChatRole::User,
            content: "Fix the build".to_owned(),
            tool_name: None,
            status: None,
            created_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&event).expect("json");
        let parsed: ChatEvent = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed.event_id, "evt-123");
        assert_eq!(parsed.role, ChatRole::User);
    }

    #[test]
    fn routing_policy_round_trip() {
        assert_eq!(RoutingPolicy::default(), RoutingPolicy::AskFirst);
        for p in [
            RoutingPolicy::AskFirst,
            RoutingPolicy::Automatic,
            RoutingPolicy::DenyCrossProvider,
        ] {
            assert_eq!(RoutingPolicy::parse_policy(p.as_str()), Some(p));
        }
        assert_eq!(RoutingPolicy::parse_policy("unknown"), None);
    }

    #[test]
    fn automatic_handoff_requires_policy_and_known_provider() {
        assert!(!RoutingPolicy::AskFirst.allows_automatic(Some("openai"), Some("openai")));
        assert!(RoutingPolicy::Automatic.allows_automatic(Some("openai"), Some("anthropic")));
        assert!(
            !RoutingPolicy::DenyCrossProvider.allows_automatic(Some("openai"), Some("anthropic"))
        );
        assert!(!RoutingPolicy::DenyCrossProvider.allows_automatic(None, None));
        assert!(RoutingPolicy::DenyCrossProvider.allows_automatic(Some("openai"), Some("openai")));
    }

    #[test]
    fn handoff_packet_serialization() {
        let packet = HandoffPacket {
            session_id: "s1".to_owned(),
            source_model: "claude-3-5-sonnet".to_owned(),
            target_model: "o3".to_owned(),
            prompt_summary: "Refactor core types".to_owned(),
            touched_files: vec!["crates/core/src/lib.rs".to_owned()],
            turn_count: 5,
            created_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&packet).expect("serialize");
        let parsed: HandoffPacket = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.target_model, "o3");
        assert_eq!(parsed.touched_files.len(), 1);
    }

    #[test]
    fn remote_pairing_verify_code_and_frame_serialization() {
        let secret = [42u8; 32];
        let code1 = RemotePairingSession::derive_verify_code(&secret);
        let code2 = RemotePairingSession::derive_verify_code(&secret);
        assert_eq!(code1, code2);
        assert_eq!(code1.len(), 6);
        assert!(code1.chars().all(|c| c.is_ascii_digit()));

        let frame = EncryptedRemoteFrame {
            sequence_num: 1,
            nonce: "0102030405060708090a0b0c".to_owned(),
            ciphertext: "deadbeefcafe".to_owned(),
        };
        let json = serde_json::to_string(&frame).expect("frame json");
        let parsed: EncryptedRemoteFrame = serde_json::from_str(&json).expect("parse frame");
        assert_eq!(parsed.sequence_num, 1);
        assert_eq!(parsed.ciphertext, "deadbeefcafe");
    }
}
