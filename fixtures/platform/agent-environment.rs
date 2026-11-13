//! Synthetic test-only agent; never reads native agent configuration.
fn main() -> std::io::Result<()> {
    let path = std::env::args().nth(1).ok_or_else(|| std::io::Error::other("report path missing"))?;
    let clean = ["CLAUDECODE", "CLAUDE_CODE_SESSION_ID", "CLAUDE_CODE_CHILD_SESSION"].iter().all(|key| std::env::var_os(key).is_none());
    let provider = std::env::var("CLAUDE_CODE_USE_BEDROCK").ok().as_deref() == Some("test-provider");
    let restriction = std::env::var("CLAUDE_CODE_CHROME_MCP_ORG_DENIED").ok().as_deref() == Some("test-denied");
    std::fs::write(path, format!("clean={clean};provider={provider};restriction={restriction}"))?;
    std::thread::sleep(std::time::Duration::from_secs(30));
    Ok(())
}
