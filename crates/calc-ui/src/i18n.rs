//! UI strings via Fluent (ARC-008). English (US) ships in 1.0; other languages are a
//! translated copy of `assets/i18n/en-US/main.ftl`.

use calc_core::CalcError;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::langid;

const EN_US: &str = include_str!("../../../assets/i18n/en-US/main.ftl");

thread_local! {
    static BUNDLE: FluentBundle<FluentResource> = bundle();
}

fn bundle() -> FluentBundle<FluentResource> {
    let res = FluentResource::try_new(EN_US.to_string()).unwrap_or_else(|(res, errors)| {
        tracing::error!("en-US.ftl: {errors:?}");
        res
    });
    let mut b = FluentBundle::new(vec![langid!("en-US")]);
    // Unicode isolation marks around placeables only matter for mixed-direction text.
    b.set_use_isolating(false);
    if let Err(e) = b.add_resource(res) {
        tracing::error!("en-US.ftl: {e:?}");
    }
    b
}

/// The translation of `id`, or `id` itself when it is missing.
pub fn tr(id: &str) -> String {
    tr_args(id, &[])
}

/// The translation of `id` with `{ $name }` placeables filled in.
pub fn tr_args(id: &str, args: &[(&str, FluentValue)]) -> String {
    BUNDLE.with(|b| {
        let Some(pattern) = b.get_message(id).and_then(|m| m.value()) else {
            return id.to_string();
        };
        let mut fargs = FluentArgs::new();
        for (k, v) in args {
            fargs.set(*k, v.clone());
        }
        let mut errors = Vec::new();
        let s = b.format_pattern(pattern, Some(&fargs), &mut errors);
        if !errors.is_empty() {
            tracing::warn!("translation {id}: {errors:?}");
        }
        s.into_owned()
    })
}

/// A number argument.
pub fn num(v: impl Into<f64>) -> FluentValue<'static> {
    FluentValue::from(v.into())
}

/// The display text of an engine error (ENG-006).
pub fn error_text(e: CalcError) -> String {
    tr(match e {
        CalcError::DivideByZero => "error-divide-by-zero",
        CalcError::Undefined => "error-undefined",
        CalcError::InvalidInput => "error-invalid-input",
        CalcError::Overflow => "error-overflow",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_parses_without_errors() {
        assert!(FluentResource::try_new(EN_US.to_string()).is_ok());
    }

    #[test]
    fn messages_format() {
        assert_eq!(tr("menu-view"), "View");
        assert_eq!(tr("no-such-key"), "no-such-key");
        assert_eq!(tr_args("stat-count", &[("count", num(3))]), "Count = 3");
        assert_eq!(tr_args("date-days", &[("days", num(1))]), "1 day");
        assert_eq!(tr_args("date-days", &[("days", num(5))]), "5 days");
        assert_eq!(error_text(CalcError::Overflow), "Overflow");
        // The engine's own error text and the UI's must agree (reference texts).
        for e in [
            CalcError::DivideByZero,
            CalcError::Undefined,
            CalcError::InvalidInput,
            CalcError::Overflow,
        ] {
            assert_eq!(error_text(e), e.to_string());
        }
    }
}
