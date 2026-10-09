//! Sums and conversions through `fend-core`, offline (`p10-spotlight.md` choice 14).

use std::time::{Duration, Instant};

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source};

/// How long an evaluation may run before it is given up.
const LIMIT: Duration = Duration::from_millis(100);

struct Deadline<'a> {
    until: Instant,
    cx: &'a Context<'a>,
    query: &'a Query,
}

impl fend_core::Interrupt for Deadline<'_> {
    fn should_interrupt(&self) -> bool {
        Instant::now() > self.until || self.cx.stale(self.query)
    }
}

/// The answer to `expression`, or `None` where it is not one fend reads.
pub fn evaluate(expression: &str, interrupt: &impl fend_core::Interrupt) -> Option<String> {
    let mut context = fend_core::Context::new();
    let result = fend_core::evaluate_with_interrupt(expression, &mut context, interrupt).ok()?;
    let answer = result.get_main_result().trim();
    (!answer.is_empty() && answer != expression.trim()).then(|| answer.to_owned())
}

pub struct CalculatorSource;

impl Source for CalculatorSource {
    fn group(&self) -> Group {
        Group::Calculation
    }

    fn answers(&self, scope: &Scope) -> bool {
        matches!(scope, Scope::General | Scope::Calculator)
    }

    fn slow(&self) -> bool {
        true
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        let text = query.text.trim();
        if text.is_empty() || (query.scope == Scope::General && !crate::reads_as_sum(text)) {
            return Vec::new();
        }
        let deadline = Deadline {
            until: Instant::now() + LIMIT,
            cx,
            query,
        };
        let Some(answer) = evaluate(text, &deadline) else {
            return Vec::new();
        };
        let Some(m) = cx.matcher.find("", "") else {
            return Vec::new();
        };
        vec![Row {
            key: "calc".into(),
            group: Group::Calculation,
            title: answer.clone(),
            detail: Some(text.to_owned()),
            trailing: None,
            icon: Icon::Glyph("calculator"),
            item: Item::Answer(answer),
            section: None,
            rank: crate::Rank::new(&m, 0.0, false, ""),
            matched: Vec::new(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Never;

    impl fend_core::Interrupt for Never {
        fn should_interrupt(&self) -> bool {
            false
        }
    }

    #[test]
    fn sums_and_conversions() {
        assert_eq!(evaluate("12*48", &Never).as_deref(), Some("576"));
        assert_eq!(evaluate("2 * (3 + 4)", &Never).as_deref(), Some("14"));
        assert!(evaluate("5 km to miles", &Never).is_some_and(|a| a.ends_with("miles")));
        assert_eq!(evaluate("firefox", &Never), None);
        assert_eq!(evaluate("12", &Never), None, "a bare number is no answer");
    }
}
