use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ── Urgency ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for Urgency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Urgency::Low => write!(f, "low"),
            Urgency::Medium => write!(f, "medium"),
            Urgency::High => write!(f, "high"),
            Urgency::Critical => write!(f, "critical"),
        }
    }
}

impl Urgency {
    pub fn from_deviation(deviation: f64, tolerance: f64) -> Self {
        let ratio = deviation / tolerance.max(f64::EPSILON);
        if ratio < 1.5 {
            Urgency::Low
        } else if ratio < 3.0 {
            Urgency::Medium
        } else if ratio < 5.0 {
            Urgency::High
        } else {
            Urgency::Critical
        }
    }
}

// ── Correction ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub metric: String,
    pub action: String,
    pub magnitude: f64,
    pub urgency: Urgency,
}

impl Correction {
    pub fn new(metric: impl Into<String>, action: impl Into<String>, magnitude: f64, urgency: Urgency) -> Self {
        Self {
            metric: metric.into(),
            action: action.into(),
            magnitude,
            urgency,
        }
    }

    pub fn description(&self) -> String {
        format!(
            "[{}] {} {} by {:.4} (urgency: {})",
            self.urgency, self.action, self.metric, self.magnitude, self.urgency
        )
    }
}

// ── Alert ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub metric: String,
    pub message: String,
    pub severity: Urgency,
    pub timestamp: u64,
    pub acknowledged: bool,
}

impl Alert {
    pub fn new(id: impl Into<String>, metric: impl Into<String>, message: impl Into<String>, severity: Urgency, timestamp: u64) -> Self {
        Self {
            id: id.into(),
            metric: metric.into(),
            message: message.into(),
            severity,
            timestamp,
            acknowledged: false,
        }
    }
}

// ── VitalSigns ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitalSigns {
    pub energy: f64,
    pub attention: f64,
    pub stress: f64,
    pub conservation_budget: f64,
    pub response_time_ms: f64,
}

impl Default for VitalSigns {
    fn default() -> Self {
        Self {
            energy: 80.0,
            attention: 0.8,
            stress: 0.1,
            conservation_budget: 50.0,
            response_time_ms: 100.0,
        }
    }
}

impl VitalSigns {
    pub fn to_readings(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("energy".into(), self.energy);
        m.insert("attention".into(), self.attention);
        m.insert("stress".into(), self.stress);
        m.insert("conservation_budget".into(), self.conservation_budget);
        m.insert("response_time_ms".into(), self.response_time_ms);
        m
    }

    pub fn from_readings(readings: &HashMap<String, f64>) -> Self {
        Self {
            energy: readings.get("energy").copied().unwrap_or(80.0),
            attention: readings.get("attention").copied().unwrap_or(0.8),
            stress: readings.get("stress").copied().unwrap_or(0.1),
            conservation_budget: readings.get("conservation_budget").copied().unwrap_or(50.0),
            response_time_ms: readings.get("response_time_ms").copied().unwrap_or(100.0),
        }
    }
}

// ── StressAction ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StressAction {
    Normal,
    Accelerated,
    Conservative,
    Emergency,
}
