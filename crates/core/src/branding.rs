//! Product identity shared with the website and desktop shell.

use std::sync::LazyLock;

/// Canonical product identity and public service locations.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Branding {
    /// User-visible product name.
    pub product_name: String,
    /// Public website base URL.
    pub base_url: String,
    /// Private source repository URL.
    pub source_repository: String,
    /// Binary releases repository URL.
    pub releases_repository: String,
    /// Default WebSocket relay URL for paired Remote (overridable via `GNOMCODE_REMOTE_RELAY_URL`).
    pub remote_relay_url: String,
    /// Default Supabase project URL for optional account sync (overridable via `GNOMCODE_SUPABASE_URL`).
    pub supabase_url: String,
    /// Supabase publishable (anon) key for optional account sync (overridable via `GNOMCODE_SUPABASE_KEY`).
    pub supabase_publishable_key: String,
}

/// Parses the bundled branding configuration without performing I/O.
pub fn branding() -> Option<&'static Branding> {
    static VALUE: LazyLock<Option<Branding>> =
        LazyLock::new(|| serde_json::from_str(include_str!("../../../web/branding.json")).ok());
    VALUE.as_ref()
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_branding_is_valid_and_uses_an_https_origin() {
        let branding = super::branding().expect("valid bundled branding");
        assert_eq!(branding.product_name, "Gnomcode");
        assert!(branding.base_url.starts_with("https://"));
        assert!(!branding.base_url.ends_with('/'));
        assert!(branding.remote_relay_url.starts_with("wss://"));
        assert!(branding.supabase_url.starts_with("https://"));
        assert!(!branding.supabase_publishable_key.is_empty());
    }
}
