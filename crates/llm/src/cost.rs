//! Rough USD cost estimation from token counts. Rates are approximate and
//! intended for budgeting/telemetry, not billing. Prices are USD per 1M tokens.

struct Rate {
    prompt: f64,
    completion: f64,
}

/// Best-effort rate lookup by model name prefix. Unknown models fall back to a
/// conservative default so cost is never silently zero.
fn rate_for(model: &str) -> Rate {
    let m = model.to_ascii_lowercase();
    if m.starts_with("gpt-4o-mini") {
        Rate { prompt: 0.15, completion: 0.60 }
    } else if m.starts_with("gpt-4o") {
        Rate { prompt: 2.50, completion: 10.00 }
    } else if m.starts_with("gpt-4.1-mini") {
        Rate { prompt: 0.40, completion: 1.60 }
    } else if m.starts_with("gpt-4.1") {
        Rate { prompt: 2.00, completion: 8.00 }
    } else {
        Rate { prompt: 0.50, completion: 1.50 }
    }
}

/// Estimated USD cost for a call with the given token counts.
pub fn estimate_cost(model: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
    let rate = rate_for(model);
    let prompt = (prompt_tokens.max(0) as f64) / 1_000_000.0 * rate.prompt;
    let completion = (completion_tokens.max(0) as f64) / 1_000_000.0 * rate.completion;
    prompt + completion
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_model_cost() {
        // 1M prompt + 1M completion on gpt-4o-mini = 0.15 + 0.60.
        let cost = estimate_cost("gpt-4o-mini", 1_000_000, 1_000_000);
        assert!((cost - 0.75).abs() < 1e-9);
    }

    #[test]
    fn unknown_model_uses_default_nonzero() {
        assert!(estimate_cost("some-future-model", 1_000_000, 0) > 0.0);
    }
}
