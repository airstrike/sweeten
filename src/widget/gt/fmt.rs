//! Number formatters for [`Table`](super::Table) cells.
//!
//! A [`Formatter`] is `Arc<dyn Fn(&Cell) -> String + Send + Sync>` — it
//! receives the full [`Cell`] (so it can choose how to render `Empty`
//! and pass `Text` through unchanged) and returns the rendered string.
//! Built-in formatters in this module render `Cell::Number` per their
//! rule, render `Cell::Empty` as `"—"`, and pass `Cell::Text` through
//! verbatim.

use std::sync::Arc;

use super::Cell;

/// A boxed cell-to-string formatter.
pub type Formatter = Arc<dyn Fn(&Cell) -> String + Send + Sync>;

/// The string rendered for [`Cell::Empty`] by the built-in numeric
/// formatters.
pub const EMPTY_GLYPH: &str = "—";

/// Integer formatting with en-US thousands separators: `1234` →
/// `"1,234"`. Negative values render with a leading minus.
pub fn number() -> Formatter {
    Arc::new(|cell| match cell {
        Cell::Number(n) => format_with_commas(*n, 0),
        Cell::Text(s) => s.clone(),
        Cell::Empty => EMPTY_GLYPH.to_string(),
    })
}

/// Fixed-decimal formatting: `1.234` → `"1.23"` for `decimals = 2`.
/// Includes en-US thousands separators on the integer part.
pub fn decimal(decimals: u8) -> Formatter {
    Arc::new(move |cell| match cell {
        Cell::Number(n) => format_with_commas(*n, decimals),
        Cell::Text(s) => s.clone(),
        Cell::Empty => EMPTY_GLYPH.to_string(),
    })
}

/// Currency formatting with en-US thousands separators:
/// `currency("$", 2)` renders `1234.5` as `"$1,234.50"`. Negatives
/// render as `"-$1,234.50"`.
pub fn currency(symbol: &str, decimals: u8) -> Formatter {
    let symbol = symbol.to_string();
    Arc::new(move |cell| match cell {
        Cell::Number(n) => {
            let body = format_with_commas(n.abs(), decimals);
            if *n < 0.0 {
                format!("-{symbol}{body}")
            } else {
                format!("{symbol}{body}")
            }
        }
        Cell::Text(s) => s.clone(),
        Cell::Empty => EMPTY_GLYPH.to_string(),
    })
}

/// Percent formatting: `0.157` → `"15.7%"` for `decimals = 1`. Multiplies
/// the input by 100 before formatting.
pub fn percent(decimals: u8) -> Formatter {
    Arc::new(move |cell| match cell {
        Cell::Number(n) => {
            format!("{}%", format_with_commas(n * 100.0, decimals))
        }
        Cell::Text(s) => s.clone(),
        Cell::Empty => EMPTY_GLYPH.to_string(),
    })
}

/// Scientific notation: `1234.0` → `"1.23e3"` for `decimals = 2`.
pub fn scientific(decimals: u8) -> Formatter {
    Arc::new(move |cell| match cell {
        Cell::Number(n) => format!("{:.*e}", decimals as usize, n),
        Cell::Text(s) => s.clone(),
        Cell::Empty => EMPTY_GLYPH.to_string(),
    })
}

/// Scaled output: `scaled(1e6, "M", 1)` renders `12_500_000` as
/// `"12.5M"`. The integer part is rendered with en-US thousands
/// separators.
pub fn scaled(divisor: f64, suffix: &str, decimals: u8) -> Formatter {
    let suffix = suffix.to_string();
    Arc::new(move |cell| match cell {
        Cell::Number(n) => {
            format!("{}{suffix}", format_with_commas(n / divisor, decimals))
        }
        Cell::Text(s) => s.clone(),
        Cell::Empty => EMPTY_GLYPH.to_string(),
    })
}

/// Wraps `inner` so that negative numeric values render in parentheses
/// and the inner formatter runs on the absolute value: `-1.5` → `"(1.5)"`.
/// Non-numeric variants pass through `inner` unchanged.
pub fn parens_for_negatives(inner: Formatter) -> Formatter {
    Arc::new(move |cell| match cell {
        Cell::Number(n) if *n < 0.0 => {
            format!("({})", inner(&Cell::Number(-n)))
        }
        Cell::Number(_) | Cell::Text(_) | Cell::Empty => inner(cell),
    })
}

/// Wraps an arbitrary closure into a [`Formatter`]. The closure receives
/// the full [`Cell`] and chooses how to render every variant.
pub fn arbitrary(
    f: impl Fn(&Cell) -> String + 'static + Send + Sync,
) -> Formatter {
    Arc::new(f)
}

fn format_with_commas(value: f64, decimals: u8) -> String {
    if !value.is_finite() {
        return format!("{value}");
    }
    let abs = value.abs();
    let formatted = format!("{:.*}", decimals as usize, abs);
    let (int_part, frac_part) = match formatted.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (formatted.as_str(), None),
    };
    let mut reversed =
        String::with_capacity(int_part.len() + int_part.len() / 3);
    for (i, ch) in int_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            reversed.push(',');
        }
        reversed.push(ch);
    }
    let int_with_commas: String = reversed.chars().rev().collect();
    let mut result = if value < 0.0 {
        format!("-{int_with_commas}")
    } else {
        int_with_commas
    };
    if let Some(f) = frac_part {
        result.push('.');
        result.push_str(f);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_emits_en_us_thousands_separators() {
        let f = decimal(2);
        assert_eq!(f(&Cell::Number(1234.5)), "1,234.50");
        assert_eq!(f(&Cell::Number(-1234.5)), "-1,234.50");
        assert_eq!(f(&Cell::Number(0.0)), "0.00");
    }

    #[test]
    fn parens_for_negatives_wraps_negative_numbers() {
        let f = parens_for_negatives(decimal(1));
        assert_eq!(f(&Cell::Number(1.5)), "1.5");
        assert_eq!(f(&Cell::Number(-1.5)), "(1.5)");
        assert_eq!(f(&Cell::Empty), EMPTY_GLYPH);
    }

    #[test]
    fn percent_multiplies_by_one_hundred() {
        let f = percent(1);
        assert_eq!(f(&Cell::Number(0.157)), "15.7%");
    }

    #[test]
    fn scaled_divides_and_appends_suffix() {
        let f = scaled(1e6, "M", 1);
        assert_eq!(f(&Cell::Number(12_500_000.0)), "12.5M");
    }

    #[test]
    fn currency_prefixes_symbol_and_handles_negatives() {
        let f = currency("$", 2);
        assert_eq!(f(&Cell::Number(1234.5)), "$1,234.50");
        assert_eq!(f(&Cell::Number(-1234.5)), "-$1,234.50");
    }

    #[test]
    fn text_passes_through_numeric_formatters() {
        let f = decimal(2);
        assert_eq!(f(&Cell::Text("n/a".to_string())), "n/a");
    }
}
