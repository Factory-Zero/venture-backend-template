//! Smoke test: catches broken module/venture composition before it ships.

#[test]
fn harness_builds() {
    // `harness()` panics (via `.expect()`) if composition is invalid, so a
    // plain call that returns is the assertion.
    let _harness = venture_backend_template::harness();
}
