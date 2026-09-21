//! Shared harness composition.
//!
//! `cratefield_cli::main_for` requires a plain zero-argument function
//! pointer, which means it cannot close over a live Cloudflare Workers
//! `Env`. So the venture's module/venture composition lives here, behind a
//! `compose()` helper that both the `fz` CLI binary (via `harness()`) and
//! the Worker's per-request `instance()` in `src/lib.rs` call, keeping the
//! module list defined in exactly one place.
//!
//! The `venture_*`/`cors_origins` helpers below read `std::env::var`, which
//! works fine when the `fz` binary runs natively (e.g. `migrations
//! collect`, `doctor`), but is always empty on Cloudflare Workers - Workers
//! never populates `std::env`, only the `Env` binding (`env.var`/
//! `env.secret`) is populated there, driven by `wrangler.toml`'s `[vars]`.
//! The defaults here are intentionally the same values `wrangler.toml`
//! ships with; if you change one, change the other.

use std::sync::Arc;

use async_trait::async_trait;
use cratefield_core::{Harness, MailError, Mailer, Message, SendOutcome, Venture};
use cratefield_module_email_signup::EmailSignup;
use cratefield_module_waitlist::Waitlist;
use cratefield_runtime_cloudflare::Cloudflare;

fn venture_name() -> String {
    std::env::var("VENTURE_NAME").unwrap_or_else(|_| "venture".to_owned())
}

fn venture_domain() -> String {
    std::env::var("VENTURE_DOMAIN").unwrap_or_else(|_| "example.com".to_owned())
}

fn cors_origins() -> Vec<String> {
    match std::env::var("CORS_ORIGINS") {
        Ok(value) if !value.is_empty() => value
            .split(',')
            .map(|origin| origin.trim().to_owned())
            .collect(),
        _ => vec![format!("https://{}", venture_domain())],
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
pub(crate) fn compose(runtime: Cloudflare) -> Harness {
    let domain = venture_domain();
    let public_url = format!("https://{domain}");

    Harness::builder()
        .venture(
            Venture::new(venture_name(), domain)
                .public_url(public_url.clone())
                .cors_origins(cors_origins()),
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
    compose(runtime)
}
