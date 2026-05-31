use crate::types::{Correction, Urgency};
use serde::{Deserialize, Serialize};

// ── Regulator trait ──────────────────────────────────────────────────

pub trait Regulator: Send + Sync {
    fn metric_name(&self) -> &str;
    fn regulate(&self, current: f64, setpoint: f64) -> Correction;
}

// ── PIDRegulator ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PIDRegulator {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    #[serde(skip)]
    pub integral: f64,
    #[serde(skip)]
    pub prev_error: f64,
}

impl PIDRegulator {
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
        }
    }

    /// Mutable regulate for simulation — updates internal state.
    pub fn regulate_mut(&mut self, current: f64, setpoint: f64) -> Correction {
        let error = setpoint - current;
        self.integral += error;
        let derivative = error - self.prev_error;
        self.prev_error = error;

        let output = self.kp * error + self.ki * self.integral + self.kd * derivative;
        let urgency = Urgency::from_deviation(error.abs(), 1.0);

        Correction::new("", if output > 0.0 { "increase" } else { "decrease" }, output.abs(), urgency)
    }

    /// Simulate PID convergence: returns final value after `steps` iterations.
    pub fn simulate_convergence(&mut self, setpoint: f64, initial: f64, steps: usize) -> f64 {
        let mut current = initial;
        for _ in 0..steps {
            let correction = self.regulate_mut(current, setpoint);
            current += correction.magnitude * if correction.action == "increase" { 1.0 } else { -1.0 };
            // Clamp integral windup
            self.integral = self.integral.clamp(-1000.0, 1000.0);
        }
        current
    }
}

impl Regulator for PIDRegulator {
    fn metric_name(&self) -> &str {
        ""
    }

    fn regulate(&self, current: f64, setpoint: f64) -> Correction {
        let error = setpoint - current;
        let urgency = Urgency::from_deviation(error.abs(), 1.0);
        let magnitude = (self.kp * error).abs();
        Correction::new("", if error > 0.0 { "increase" } else { "decrease" }, magnitude, urgency)
    }
}

// ── BangBangRegulator ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangBangRegulator {
    pub threshold: f64,
    pub metric: String,
}

impl BangBangRegulator {
    pub fn new(metric: impl Into<String>, threshold: f64) -> Self {
        Self {
            threshold,
            metric: metric.into(),
        }
    }
}

impl Regulator for BangBangRegulator {
    fn metric_name(&self) -> &str {
        &self.metric
    }

    fn regulate(&self, current: f64, setpoint: f64) -> Correction {
        let lower = setpoint - self.threshold;
        let upper = setpoint + self.threshold;

        if current < lower {
            Correction::new(&self.metric, "turn_on", (setpoint - current).abs(), Urgency::High)
        } else if current > upper {
            Correction::new(&self.metric, "turn_off", (current - setpoint).abs(), Urgency::High)
        } else {
            Correction::new(&self.metric, "maintain", 0.0, Urgency::Low)
        }
    }
}

// ── ThresholdRegulator ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdRegulator {
    pub metric: String,
    /// (low_bound, high_bound, action) — zones sorted by low_bound
    pub zones: Vec<(f64, f64, String)>,
}

impl ThresholdRegulator {
    pub fn new(metric: impl Into<String>, zones: Vec<(f64, f64, String)>) -> Self {
        let mut zones = zones;
        zones.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Self {
            metric: metric.into(),
            zones,
        }
    }
}

impl Regulator for ThresholdRegulator {
    fn metric_name(&self) -> &str {
        &self.metric
    }

    fn regulate(&self, current: f64, _setpoint: f64) -> Correction {
        for (low, high, action) in &self.zones {
            if *low <= current && current < *high {
                return Correction::new(&self.metric, action, 0.0, Urgency::Medium);
            }
        }
        Correction::new(&self.metric, "unknown_zone", 0.0, Urgency::Critical)
    }
}
