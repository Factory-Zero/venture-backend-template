#![forbid(unsafe_code)]

mod harness;

use std::sync::{Arc, OnceLock};

use cratefield_adapter_resend::Resend;
use cratefield_adapter_turnstile::Turnstile;
use cratefield_core::{Harness, Mailer};
use cratefield_runtime_cloudflare::{
    Cloudflare, FetchClient, WorkersClock, serve, serve_scheduled,
};
pub use harness::harness;
use worker::{Context, Env, Request, Response, event};

fn settings(env: &Env) -> harness::Settings {
    harness::Settings::from_lookup(|key| env.var(key).ok().map(|value| value.to_string()))
}

fn build_captcha(env: &Env) -> Option<Turnstile> {
    let secret = env
        .secret("TURNSTILE_SECRET")
        .ok()
        .map(|secret| secret.to_string())
        .filter(|secret| !secret.is_empty())?;
    Some(
        Turnstile::new(Arc::new(FetchClient), Arc::new(WorkersClock), secret)
            .expected_hostname(&settings(env).domain),
    )
}

fn build_mailer(env: &Env) -> Arc<dyn Mailer> {
    let key = env
        .secret("RESEND_API_KEY")
        .ok()
        .map(|secret| secret.to_string())
        .filter(|key| !key.is_empty());
    match key {
        Some(key) => {
            let mail_from = env
                .var("MAIL_FROM")
                .ok()
                .map(|value| value.to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "no-reply@example.com".to_owned());
            Arc::new(Resend::new(
                Arc::new(FetchClient),
                Arc::new(WorkersClock),
                Some(key),
                &mail_from,
                None,
            ))
        }
        None => Arc::new(harness::NoopMailer),
    }
}

static INSTANCE: OnceLock<(Harness, Cloudflare)> = OnceLock::new();

fn instance(env: &Env) -> &'static (Harness, Cloudflare) {
    INSTANCE.get_or_init(|| {
        let mailer = build_mailer(env);
        let mut build_runtime = Cloudflare::new()
            .db("DB")
            .mailer_arc(Arc::clone(&mailer))
            .rate_limiter("RATE_LIMITER");
        if let Some(captcha) = build_captcha(env) {
            build_runtime = build_runtime.captcha(captcha);
        }
        let harness = harness::compose(build_runtime, settings(env));

        let mut runtime = Cloudflare::new()
            .db("DB")
            .mailer_arc(mailer)
            .rate_limiter("RATE_LIMITER");
        if let Some(captcha) = build_captcha(env) {
            runtime = runtime.captcha(captcha);
        }
        (harness, runtime)
    })
}

#[event(fetch)]
pub async fn fetch(req: Request, env: Env, ctx: Context) -> worker::Result<Response> {
    let (harness, runtime) = instance(&env);
    serve(harness, runtime, req, env, ctx).await
}

#[event(scheduled)]
pub async fn scheduled(event: worker::ScheduledEvent, env: Env, ctx: worker::ScheduleContext) {
    let (harness, runtime) = instance(&env);
    serve_scheduled(harness, runtime, event, env, ctx).await;
}
