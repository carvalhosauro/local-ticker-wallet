use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::core::types::Side;

/// Which field has focus in the add-transaction form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddField {
    #[default]
    Symbol,
    Side,
    Quantity,
    Price,
    Date,
    Fees,
    Note,
}

impl AddField {
    pub fn next(self) -> Self {
        match self {
            Self::Symbol => Self::Side,
            Self::Side => Self::Quantity,
            Self::Quantity => Self::Price,
            Self::Price => Self::Date,
            Self::Date => Self::Fees,
            Self::Fees => Self::Note,
            Self::Note => Self::Symbol,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Symbol => Self::Note,
            Self::Side => Self::Symbol,
            Self::Quantity => Self::Side,
            Self::Price => Self::Quantity,
            Self::Date => Self::Price,
            Self::Fees => Self::Date,
            Self::Note => Self::Fees,
        }
    }
}

/// In-progress add-transaction form (overlay state).
#[derive(Debug, Clone)]
pub struct AddTransactionForm {
    pub symbol: String,
    pub side: Side,
    pub quantity: String,
    pub price: String,
    pub date: String,
    pub fees: String,
    pub note: String,
    pub focused: AddField,
    pub error: Option<String>,
    /// When true, the next typed character replaces the entire field value.
    pub replace_on_input: bool,
}

impl AddTransactionForm {
    pub fn new(symbol: Option<String>, price_hint: Option<String>) -> Self {
        let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
        let has_symbol = symbol.is_some();
        Self {
            symbol: symbol.unwrap_or_default(),
            side: Side::Buy,
            quantity: String::new(),
            price: price_hint.unwrap_or_default(),
            date: today,
            fees: "0".into(),
            note: String::new(),
            focused: if has_symbol {
                AddField::Quantity
            } else {
                AddField::Symbol
            },
            error: None,
            replace_on_input: false,
        }
    }

    pub fn focused_mut(&mut self) -> &mut String {
        match self.focused {
            AddField::Symbol => &mut self.symbol,
            AddField::Quantity => &mut self.quantity,
            AddField::Price => &mut self.price,
            AddField::Date => &mut self.date,
            AddField::Fees => &mut self.fees,
            AddField::Note => &mut self.note,
            AddField::Side => &mut self.symbol,
        }
    }

    pub fn toggle_side(&mut self) {
        self.side = match self.side {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        };
    }
}

/// Normalizes user decimal input (accepts comma or dot).
pub fn normalize_decimal(s: &str) -> String {
    s.trim().replace(',', ".")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTransaction {
    pub symbol: String,
    pub side: Side,
    pub quantity: Decimal,
    pub price: Decimal,
    pub fees: Decimal,
    pub executed_at: NaiveDate,
    pub note: Option<String>,
}

/// Validates form fields; returns typed payload or error field key.
pub fn validate(form: &AddTransactionForm) -> Result<ValidatedTransaction, &'static str> {
    let symbol = form.symbol.trim().to_uppercase();
    if symbol.is_empty() {
        return Err("symbol");
    }

    let quantity: Decimal = normalize_decimal(&form.quantity).parse().map_err(|_| "quantity")?;
    if quantity <= Decimal::ZERO {
        return Err("quantity");
    }

    let price: Decimal = normalize_decimal(&form.price).parse().map_err(|_| "price")?;
    if price < Decimal::ZERO {
        return Err("price");
    }

    let fees: Decimal = normalize_decimal(&form.fees).parse().map_err(|_| "fees")?;
    if fees < Decimal::ZERO {
        return Err("fees");
    }

    let executed_at =
        NaiveDate::parse_from_str(form.date.trim(), "%Y-%m-%d").map_err(|_| "date")?;

    let note = {
        let n = form.note.trim();
        if n.is_empty() {
            None
        } else {
            Some(n.to_string())
        }
    };

    Ok(ValidatedTransaction {
        symbol,
        side: form.side,
        quantity,
        price,
        fees,
        executed_at,
        note,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn validate_accepts_comma_decimals() {
        let form = AddTransactionForm {
            symbol: "petr4".into(),
            side: Side::Buy,
            quantity: "100".into(),
            price: "28,50".into(),
            date: "2026-01-02".into(),
            fees: "0".into(),
            note: String::new(),
            focused: AddField::Symbol,
            error: None,
            replace_on_input: false,
        };
        let v = validate(&form).unwrap();
        assert_eq!(v.symbol, "PETR4");
        assert_eq!(v.price, dec!(28.50));
    }

    #[test]
    fn validate_rejects_empty_symbol() {
        let form = AddTransactionForm::new(None, None);
        assert_eq!(validate(&form).unwrap_err(), "symbol");
    }
}
