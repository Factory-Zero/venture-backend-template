//! Shared harness composition.
//!
//! `cratefield_cli::main_for` requires a plain zero-argument function
//! pointer, which means it cannot close over a live Cloudflare Workers
//! `Env`. So the venture's module/venture composition lives here, behind a
//! `compose()` helper that both the `fz` CLI binary (via `harness()`) and
//! the Worker's per-request `instance()` in `src/lib.rs` call, keeping the
//! module list defined in exactly one place.
//!
//! Venture settings (`VENTURE_NAME`, `VENTURE_DOMAIN`, `CORS_ORIGINS`) are
//! read through [`Settings::from_lookup`], so each caller supplies its own
//! source: `std::env` for the native `fz` binary, and the Worker `Env`
//! (`wrangler.toml`'s `[vars]`) at runtime, where `std::env` is always
//! empty. The defaults match the base `[vars]` in `wrangler.toml`.

use std::sync::Arc;

use async_trait::async_trait;
use cratefield_core::{Harness, MailError, Mailer, Message, SendOutcome, Venture};
use cratefield_module_email_signup::EmailSignup;
use cratefield_module_waitlist::Waitlist;
use cratefield_runtime_cloudflare::Cloudflare;

/// The venture's identity and CORS policy, read from whichever variable
/// source the caller has.
pub(crate) struct Settings {
    pub(crate) name: String,
    pub(crate) domain: String,
    pub(crate) cors_origins: Vec<String>,
}

impl Settings {
    /// Reads the settings through `lookup`, treating a missing or empty
    /// variable as unset.
    pub(crate) fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let get = |key: &str| lookup(key).filter(|value| !value.is_empty());
        let name = get("VENTURE_NAME").unwrap_or_else(|| "venture".to_owned());
        let domain = get("VENTURE_DOMAIN").unwrap_or_else(|| "example.com".to_owned());
        let cors_origins = match get("CORS_ORIGINS") {
            Some(value) => value
                .split(',')
                .map(|origin| origin.trim().to_owned())
                .filter(|origin| !origin.is_empty())
                .collect(),
            None => vec![format!("https://{domain}")],
        };
        Self {
            name,
            domain,
            cors_origins,
        }
    }
}

/// A [`Mailer`] that accepts every message without sending anything. Used
/// as the default mailer for the CLI-only `harness()` build (which never
/// handles live requests) and as a fallback when no mail provider secret is
/// configured.
pub(crate) struct NoopMailer;

#[async_trait]
impl Mailer for NoopMailer {
    async fn send(&self, _message: Message) -> Result<SendOutcome, MailError> {
        Ok(SendOutcome::Sent {
            id: "noop".to_owned(),
        })
    }
}

/// Build the `Harness` for the given runtime. This is the single place the
/// venture's modules are registered - both `harness()` (used by the `fz`
/// CLI) and the Worker's per-request instance in `src/lib.rs` call this.
pub(crate) fn compose(runtime: Cloudflare, settings: Settings) -> Harness {
    let public_url = format!("https://{}", settings.domain);

    Harness::builder()
        .venture(
            Venture::new(settings.name, settings.domain)
                .public_url(public_url.clone())
                .cors_origins(settings.cors_origins),
        )
        .templates(cratefield_module_waitlist::default_templates())
        .templates(cratefield_module_email_signup::default_templates())
        .module(Waitlist::new().any_product().status_redirect(public_url))
        .module(EmailSignup::new())
        .runtime(runtime)
        .build()
        .expect("venture harness is valid")
}

/// Build the harness with a default, CLI-only runtime (no live secrets, no
/// captcha). Used by the `fz` CLI (`migrations collect`, `doctor`) and by
/// tests - it must build successfully without any Cloudflare `Env`.
pub fn harness() -> Harness {
    let runtime = Cloudflare::new()
        .db("DB")
        .mailer_arc(Arc::new(NoopMailer))
        .rate_limiter("RATE_LIMITER");
    compose(
        runtime,
        Settings::from_lookup(|key| std::env::var(key).ok()),
    )
}

#[cfg(test)]
mod tests {
    use super::Settings;

    fn from(pairs: &[(&str, &str)]) -> Settings {
        Settings::from_lookup(|key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_owned())
        })
    }

    #[test]
    fn the_callers_variables_win_over_the_defaults() {
        let settings = from(&[
            ("VENTURE_NAME", "acme"),
            ("VENTURE_DOMAIN", "staging.acme.dev"),
            ("CORS_ORIGINS", "https://a.acme.dev, https://b.acme.dev,"),
        ]);
        assert_eq!(settings.name, "acme");
        assert_eq!(settings.domain, "staging.acme.dev");
        assert_eq!(
            settings.cors_origins,
            ["https://a.acme.dev", "https://b.acme.dev"]
        );
    }

    #[test]
    fn missing_or_empty_variables_fall_back_to_the_domain() {
        let settings = from(&[("VENTURE_DOMAIN", "acme.dev"), ("CORS_ORIGINS", "")]);
        assert_eq!(settings.name, "venture");
        assert_eq!(settings.cors_origins, ["https://acme.dev"]);
    }
}
