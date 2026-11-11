//! Dates du projet : un instant à la seconde, en temps universel.
//!
//! Écrit dans les fichiers sous la forme `2026-11-11T14:03:22Z`, lisible par
//! un humain et identique sur les deux Mac. Pas de dépendance externe : le
//! calcul du calendrier tient en quelques lignes.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::error::ProjectError;

/// Nombre de secondes écoulées depuis le 1er janvier 1970 à 0 h UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Timestamp(u64);

impl Timestamp {
    /// L'instant présent, d'après l'horloge de la machine.
    pub fn now() -> Self {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self(seconds)
    }

    pub fn from_unix_seconds(seconds: u64) -> Self {
        Self(seconds)
    }

    pub fn unix_seconds(self) -> u64 {
        self.0
    }

    /// Secondes écoulées depuis `earlier` ; zéro si `earlier` est dans le
    /// futur (horloges des deux Mac légèrement décalées).
    pub fn seconds_since(self, earlier: Timestamp) -> u64 {
        self.0.saturating_sub(earlier.0)
    }

    /// Forme écrite : `AAAA-MM-JJTHH:MM:SSZ`.
    pub fn to_iso8601(self) -> String {
        let days = (self.0 / 86_400) as i64;
        let rest = self.0 % 86_400;
        let (year, month, day) = civil_from_days(days);
        format!(
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
            rest / 3600,
            rest % 3600 / 60,
            rest % 60
        )
    }

    /// Lit la forme écrite par [`Timestamp::to_iso8601`], et elle seule.
    pub fn parse_iso8601(text: &str) -> Result<Self, ProjectError> {
        let invalid = || ProjectError::InvalidTimestamp {
            value: text.to_string(),
        };
        let b = text.as_bytes();
        if b.len() != 20
            || b[4] != b'-'
            || b[7] != b'-'
            || b[10] != b'T'
            || b[13] != b':'
            || b[16] != b':'
            || b[19] != b'Z'
        {
            return Err(invalid());
        }
        let number = |range: std::ops::Range<usize>| -> Option<u64> {
            let part = &b[range];
            if part.iter().all(u8::is_ascii_digit) {
                std::str::from_utf8(part).ok()?.parse().ok()
            } else {
                None
            }
        };
        let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
            number(0..4),
            number(5..7),
            number(8..10),
            number(11..13),
            number(14..16),
            number(17..19),
        ) else {
            return Err(invalid());
        };
        if year < 1970
            || !(1..=12).contains(&month)
            || !(1..=31).contains(&day)
            || hour > 23
            || minute > 59
            || second > 59
        {
            return Err(invalid());
        }
        let days = days_from_civil(year as i64, month as i64, day as i64);
        let parsed = Self(days as u64 * 86_400 + hour * 3600 + minute * 60 + second);
        // Refuse les jours qui n'existent pas (30 février…) : ils ne
        // survivent pas à l'aller-retour.
        if parsed.to_iso8601() != text {
            return Err(invalid());
        }
        Ok(parsed)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_iso8601())
    }
}

impl From<Timestamp> for String {
    fn from(value: Timestamp) -> Self {
        value.to_iso8601()
    }
}

impl TryFrom<String> for Timestamp {
    type Error = ProjectError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse_iso8601(&value)
    }
}

/// Date (année, mois, jour) du jour numéro `days` depuis le 1er janvier 1970.
/// Calendrier grégorien, algorithme de Howard Hinnant.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Inverse de [`civil_from_days`].
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates_are_written_as_expected() {
        assert_eq!(
            Timestamp::from_unix_seconds(0).to_iso8601(),
            "1970-01-01T00:00:00Z"
        );
        // 11 novembre 2026 à 14 h 03 min 22 s UTC.
        assert_eq!(
            Timestamp::from_unix_seconds(1_794_405_802).to_iso8601(),
            "2026-11-11T14:03:22Z"
        );
        // 29 février d'une année bissextile.
        assert_eq!(
            Timestamp::from_unix_seconds(1_709_164_800).to_iso8601(),
            "2024-02-29T00:00:00Z"
        );
    }

    #[test]
    fn written_dates_are_read_back_exactly() {
        for seconds in [
            0,
            59,
            86_399,
            86_400,
            951_782_400,
            1_709_164_800,
            1_794_405_802,
            4_102_444_799,
        ] {
            let t = Timestamp::from_unix_seconds(seconds);
            let back = Timestamp::parse_iso8601(&t.to_iso8601()).expect("date relue");
            assert_eq!(back, t);
        }
    }

    #[test]
    fn malformed_dates_are_refused() {
        for text in [
            "",
            "2026-11-11",
            "2026-11-11 14:03:22Z",
            "2026-11-11T14:03:22",
            "2026-02-30T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-11-11T24:00:00Z",
            "2026-11-11T14:03:22+01",
            "20261111T140322Z0000",
        ] {
            assert!(Timestamp::parse_iso8601(text).is_err(), "accepté : {text}");
        }
    }

    #[test]
    fn elapsed_time_never_goes_negative() {
        let early = Timestamp::from_unix_seconds(100);
        let late = Timestamp::from_unix_seconds(160);
        assert_eq!(late.seconds_since(early), 60);
        assert_eq!(early.seconds_since(late), 0);
    }
}
