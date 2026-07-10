use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;

pub const FIVE_HOUR_MINS: u64 = 5 * 60;
pub const SEVEN_DAY_MINS: u64 = 7 * 24 * 60;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
    pub connection: ConnectionStatus,
    pub error: Option<String>,
    pub updated_at: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    #[default]
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub used_percent: f64,
    pub window_duration_mins: u64,
    pub resets_at: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SharedUsageState(pub Arc<RwLock<UsageSnapshot>>);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitWindow {
    pub used_percent: f64,
    pub window_duration_mins: u64,
    pub resets_at: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitBucket {
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
}

impl UsageSnapshot {
    pub fn apply_bucket(&mut self, bucket: RateLimitBucket) {
        for window in [bucket.primary, bucket.secondary].into_iter().flatten() {
            let usage = UsageWindow {
                used_percent: window.used_percent.clamp(0.0, 100.0),
                window_duration_mins: window.window_duration_mins,
                resets_at: window.resets_at,
            };

            match usage.window_duration_mins {
                FIVE_HOUR_MINS => self.five_hour = Some(usage),
                SEVEN_DAY_MINS => self.seven_day = Some(usage),
                _ => {}
            }
        }

        self.connection = ConnectionStatus::Connected;
        self.error = None;
        self.updated_at = Some(now_unix());
    }

    pub fn set_connecting(&mut self) {
        self.connection = ConnectionStatus::Connecting;
        self.error = None;
    }

    pub fn set_error(&mut self, message: String) {
        self.connection = ConnectionStatus::Error;
        self.error = Some(message);
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(duration: u64, used: f64) -> RateLimitWindow {
        RateLimitWindow {
            used_percent: used,
            window_duration_mins: duration,
            resets_at: 1_800_000_000,
        }
    }

    #[test]
    fn selects_five_hour_and_seven_day_windows() {
        let mut snapshot = UsageSnapshot::default();
        snapshot.apply_bucket(RateLimitBucket {
            primary: Some(window(FIVE_HOUR_MINS, 32.0)),
            secondary: Some(window(SEVEN_DAY_MINS, 15.0)),
        });

        assert_eq!(snapshot.five_hour.unwrap().used_percent, 32.0);
        assert_eq!(snapshot.seven_day.unwrap().used_percent, 15.0);
        assert_eq!(snapshot.connection, ConnectionStatus::Connected);
    }

    #[test]
    fn ignores_unknown_windows_and_preserves_known_values() {
        let mut snapshot = UsageSnapshot {
            five_hour: Some(UsageWindow {
                used_percent: 20.0,
                window_duration_mins: FIVE_HOUR_MINS,
                resets_at: 1,
            }),
            ..UsageSnapshot::default()
        };

        snapshot.apply_bucket(RateLimitBucket {
            primary: Some(window(60, 80.0)),
            secondary: None,
        });

        assert_eq!(snapshot.five_hour.unwrap().used_percent, 20.0);
        assert!(snapshot.seven_day.is_none());
    }

    #[test]
    fn clamps_invalid_percentages() {
        let mut snapshot = UsageSnapshot::default();
        snapshot.apply_bucket(RateLimitBucket {
            primary: Some(window(FIVE_HOUR_MINS, 120.0)),
            secondary: Some(window(SEVEN_DAY_MINS, -5.0)),
        });

        assert_eq!(snapshot.five_hour.unwrap().used_percent, 100.0);
        assert_eq!(snapshot.seven_day.unwrap().used_percent, 0.0);
    }
}
