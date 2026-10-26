//! Domain types, pricing, quota, rules and redaction. Pure: no I/O, no async.
//!
//! See `docs/SPEC.md` for the design. This crate is a scaffold; implementation starts in M1.

/// How much a reported number can be trusted (SPEC §3.1, invariant 6).
///
/// Every cost, quota or token figure surfaced to the user carries one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrustLevel {
    /// Read directly from agent data (e.g. token counts in a log line).
    Exact,
    /// Derived by FleetHUD heuristics (e.g. Claude 5-hour window estimate).
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

#[cfg(test)]
mod tests {
    use super::TrustLevel;

    #[test]
    fn trust_level_identifiers_are_stable() {
        assert_eq!(TrustLevel::Exact.as_str(), "exact");
        assert_eq!(TrustLevel::Estimated.as_str(), "estimated");
        assert_eq!(TrustLevel::Api.as_str(), "api");
        assert_eq!(TrustLevel::Unknown.as_str(), "unknown");
    }
}
