use chrono::{Datelike, Timelike};
use chrono_tz::America::Sao_Paulo;
use chrono_tz::Tz;

pub fn is_open(now: chrono::DateTime<Tz>) -> bool {
    let wd = now.weekday();
    let is_weekday = !matches!(wd, chrono::Weekday::Sat | chrono::Weekday::Sun);
    let h = now.hour();
    is_weekday && (10..18).contains(&h)
}

pub fn now_sp() -> chrono::DateTime<Tz> {
    chrono::Utc::now().with_timezone(&Sao_Paulo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn weekday_noon_is_open() {
        // 2026-06-22 is a Monday
        let t = Sao_Paulo.with_ymd_and_hms(2026, 6, 22, 12, 0, 0).unwrap();
        assert!(is_open(t));
    }

    #[test]
    fn weekend_is_closed() {
        // 2026-06-21 is a Sunday
        let t = Sao_Paulo.with_ymd_and_hms(2026, 6, 21, 12, 0, 0).unwrap();
        assert!(!is_open(t));
    }

    #[test]
    fn before_open_is_closed() {
        let t = Sao_Paulo.with_ymd_and_hms(2026, 6, 22, 9, 0, 0).unwrap();
        assert!(!is_open(t));
    }
}
