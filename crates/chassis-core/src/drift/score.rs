//! Pure drift numeric rubric — no filesystem, git, or wall-clock access.

#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};

const CH_DRIFT_CHURN_INFO: &str = "CH-DRIFT-CHURN-INFO";
const CH_DRIFT_CLAIM_STALE: &str = "CH-DRIFT-CLAIM-STALE";
const CH_DRIFT_CLAIM_ABANDONED: &str = "CH-DRIFT-CLAIM-ABANDONED";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftKind {
    Info,
    StaleWarning,
    AbandonedError,
}

impl DriftKind {
    pub fn rule_id(self) -> &'static str {
        match self {
            DriftKind::Info => CH_DRIFT_CHURN_INFO,
            DriftKind::StaleWarning => CH_DRIFT_CLAIM_STALE,
            DriftKind::AbandonedError => CH_DRIFT_CLAIM_ABANDONED,
        }
    }

    #[inline]
    fn band_for_score(score: f64) -> Option<DriftKind> {
        if score <= f64::EPSILON {
            None
        } else if score <= 5.0 {
            Some(DriftKind::Info)
        } else if score <= 20.0 {
            Some(DriftKind::StaleWarning)
        } else {
            Some(DriftKind::AbandonedError)
        }
    }
}

#[inline]
fn days_since_claim(claim_last_edit: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
    let d = now - claim_last_edit;
    d.num_seconds().max(0) as f64 / 86_400.0_f64
}

/// ADR-0024 drift score — `churn × ln(1 + days_since_last_claim_edit)`, with explicit `now`.
pub fn score(
    claim_last_edit: DateTime<Utc>,
    _impl_last_edit: DateTime<Utc>,
    churn: usize,
    now: DateTime<Utc>,
) -> (f64, Option<DriftKind>) {
    let days = days_since_claim(claim_last_edit, now);
    let raw = churn as f64 * (1_f64 + days).ln();

    let band = DriftKind::band_for_score(raw);
    (raw, band)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn at(y: i32, mo: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, 0, 0, 0).unwrap()
    }

    #[test]
    fn zero_churn_maps_to_no_band() {
        let c = at(2026, 1, 1);
        let (_, band) = score(c, at(2026, 1, 5), 0, at(2026, 6, 1));
        assert!(band.is_none());
    }

    #[test]
    fn churn_with_small_days_info_band() {
        let claim = at(2026, 5, 1);
        let now = at(2026, 5, 2);
        let (_, band) = score(claim, now, 2, now);
        assert_eq!(band, Some(DriftKind::Info));
    }

    #[test]
    fn large_churn_large_days_abandoned() {
        let claim = at(2020, 1, 1);
        let now = at(2026, 6, 1);
        let (_s, band) = score(claim, now, 200, now);
        assert_eq!(band, Some(DriftKind::AbandonedError));
    }

    #[test]
    fn moderate_values_warning_band() {
        // ~43 days × churn 4 ⇒ raw ≈ 4 * ln(44) ≈ 15.1 ⇒ (5, 20] ⇒ StaleWarning.
        let claim = at(2026, 4, 1);
        let now = at(2026, 5, 14);
        let (_, band) = score(claim, now, 4, now);
        assert_eq!(band, Some(DriftKind::StaleWarning));
    }
}
