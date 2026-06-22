use crate::config::Locale;
use rust_decimal::Decimal;

/// Display locale for number formatting (mirrors `config::Locale`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatLocale {
    pub decimal_sep: char,
    pub thousands_sep: char,
    pub currency_prefix: &'static str,
}

impl From<Locale> for FormatLocale {
    fn from(locale: Locale) -> Self {
        match locale {
            Locale::PtBr => Self {
                decimal_sep: ',',
                thousands_sep: '.',
                currency_prefix: "R$ ",
            },
            Locale::En => Self {
                decimal_sep: '.',
                thousands_sep: ',',
                currency_prefix: "$",
            },
        }
    }
}

fn format_fixed(value: Decimal, places: u32, locale: FormatLocale) -> String {
    let rounded = value.round_dp(places);
    let negative = rounded.is_sign_negative();
    let abs = rounded.abs();

    let s = if places == 0 {
        abs.trunc().to_string()
    } else {
        format!("{:.prec$}", abs, prec = places as usize)
    };

    let (int_part, frac_part) = if let Some(dot) = s.find('.') {
        (&s[..dot], Some(&s[dot + 1..]))
    } else {
        (s.as_str(), None)
    };

    let grouped = group_digits(int_part, locale.thousands_sep);
    let mut out = if let Some(frac) = frac_part {
        format!("{}{}{}", grouped, locale.decimal_sep, frac)
    } else {
        grouped
    };
    if negative {
        out.insert(0, '-');
    }
    out
}

fn group_digits(int_part: &str, sep: char) -> String {
    let digits: String = int_part.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() <= 3 {
        return digits;
    }
    let mut out = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(sep);
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Unit price — 2 decimal places; 4 when price < 1.
pub fn format_price(value: Decimal, locale: FormatLocale) -> String {
    let places = if value.abs() < Decimal::ONE && !value.is_zero() {
        4
    } else {
        2
    };
    format_fixed(value, places, locale)
}

/// Monetary amount with currency prefix.
pub fn format_money(value: Decimal, locale: FormatLocale) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    format!("{}{}{}", sign, locale.currency_prefix, format_fixed(value.abs(), 2, locale))
}

/// Signed percentage with `%` suffix.
pub fn format_pct(value: Decimal, locale: FormatLocale) -> String {
    let sign = if value > Decimal::ZERO {
        "+"
    } else if value < Decimal::ZERO {
        "-"
    } else {
        ""
    };
    format!("{}{}%", sign, format_fixed(value.abs(), 2, locale))
}

/// Quantity — integer when whole, otherwise up to 4 decimal places.
pub fn format_quantity(value: Decimal, locale: FormatLocale) -> String {
    if value.fract().is_zero() {
        format_fixed(value, 0, locale)
    } else {
        format_fixed(value, 4, locale)
    }
}

/// Score sub-component — 1 decimal place.
pub fn format_score_sub(value: Decimal, locale: FormatLocale) -> String {
    format_fixed(value, 1, locale)
}

/// Whole score 0–100.
pub fn format_score(value: u8) -> String {
    value.to_string()
}

/// Parse a decimal string from IPC (always uses `.` separator).
pub fn parse_decimal(s: &str) -> Option<Decimal> {
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const PT: FormatLocale = FormatLocale {
        decimal_sep: ',',
        thousands_sep: '.',
        currency_prefix: "R$ ",
    };
    const EN: FormatLocale = FormatLocale {
        decimal_sep: '.',
        thousands_sep: ',',
        currency_prefix: "$",
    };

    #[test]
    fn format_price_pt_br() {
        assert_eq!(format_price(dec!(28.5), PT), "28,50");
        assert_eq!(format_price(dec!(0.3845), PT), "0,3845");
    }

    #[test]
    fn format_money_pt_br() {
        assert_eq!(format_money(dec!(8106.0), PT), "R$ 8.106,00");
        assert_eq!(format_money(dec!(-100.5), PT), "-R$ 100,50");
    }

    #[test]
    fn format_pct_en() {
        assert_eq!(format_pct(dec!(1.25), EN), "+1.25%");
        assert_eq!(format_pct(dec!(-3.4), EN), "-3.40%");
    }

    #[test]
    fn format_quantity_integer() {
        assert_eq!(format_quantity(dec!(100), PT), "100");
        assert_eq!(format_quantity(dec!(0.5), EN), "0.5000");
    }
}
