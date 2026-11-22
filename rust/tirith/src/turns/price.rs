//! A turn's cost when its agent gives none: `[prices."<model id>"]` of config.toml, in dollars per million tokens.
use super::usage::Usage;
use crate::config::get;

/// The cost of `u` with the price of `model` (`input`, `output` and `cached`, the last one `input` when missing), or none without a price: never invented.
pub fn cost(raw: &toml::Table, model: &str, u: &Usage) -> Option<f64> {
    let price = get::<toml::Table>(raw, &["prices", model])?;
    let rate = |k: &str| {
        price
            .get(k)
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
    };
    let (input, output) = (rate("input")?, rate("output")?);
    let cached = rate("cached").unwrap_or(input);
    let fresh = super::settle::fresh(u);
    let dollars = fresh as f64 * input
        + u.cache_write as f64 * input
        + u.cache_read as f64 * cached
        + u.output as f64 * output;
    Some(dollars / 1e6)
}
