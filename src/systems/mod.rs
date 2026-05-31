use crate::types::{Alert, StressAction, Urgency};
use crate::HomeostasisController;
use serde::{Deserialize, Serialize};

// ── AlertSystem ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSystem {
    pub alerts: Vec<Alert>,
    pub max_alerts: usize,
    #[serde(skip)]
    next_id: u64,
}

impl AlertSystem {
    pub fn new(max_alerts: usize) -> Self {
        Self {
            alerts: Vec::new(),
            max_alerts,
            next_id: 0,
        }
    }

    pub fn check(&mut self, controller: &HomeostasisController) {
        for (metric, &setpoint) in &controller.setpoints {
            if let Some(&current) = controller.current.get(metric) {
                let deviation = (current - setpoint).abs();
                if let Some(&tol) = controller.tolerances.get(metric) {
                    if deviation > tol {
                        let urgency = Urgency::from_deviation(deviation, tol);
                        let id = format!("alert-{}", self.next_id);
                        self.next_id += 1;
                        let msg = format!("{} deviated by {:.4} (tolerance: {:.4})", metric, deviation, tol);
                        self.alerts.push(Alert::new(id, metric, msg, urgency, 0));
                    }
                }
            }
        }
        // Trim oldest if over max
        while self.alerts.len() > self.max_alerts {
            self.alerts.remove(0);
        }
    }

    pub fn active_alerts(&self) -> Vec<&Alert> {
        self.alerts.iter().filter(|a| !a.acknowledged).collect()
    }

    pub fn critical_alerts(&self) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|a| matches!(a.severity, Urgency::Critical | Urgency::High))
            .collect()
    }

    pub fn acknowledge(&mut self, id: &str) {
        if let Some(a) = self.alerts.iter_mut().find(|a| a.id == id) {
            a.acknowledged = true;
        }
    }

    pub fn clear_resolved(&mut self, controller: &HomeostasisController) {
        self.alerts.retain(|alert| {
            let setpoint = controller.setpoints.get(&alert.metric);
            let current = controller.current.get(&alert.metric);
            let tolerance = controller.tolerances.get(&alert.metric);
            match (setpoint, current, tolerance) {
                (Some(sp), Some(cur), Some(tol)) => (cur - sp).abs() > *tol,
                _ => true, // keep if we can't determine
            }
        });
    }
}

// ── CircadianRhythm ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircadianRhythm {
    pub period_ms: u64,
    pub amplitude: f64,
    pub phase: f64,
    pub base_setpoints: std::collections::HashMap<String, f64>,
}

impl CircadianRhythm {
    pub fn new(period_ms: u64, amplitude: f64, phase: f64) -> Self {
        Self {
            period_ms,
            amplitude,
            phase,
            base_setpoints: std::collections::HashMap::new(),
        }
    }

    /// Sinusoidal modulation of a base setpoint.
    pub fn setpoint_at(&self, metric: &str, time_ms: u64) -> f64 {
        let base = self.base_setpoints.get(metric).copied().unwrap_or(0.0);
        let t = (time_ms as f64 / self.period_ms as f64) * 2.0 * std::f64::consts::PI + self.phase;
        base + self.amplitude * t.sin()
    }

    /// Whether we're in the "active" (sin > 0) phase.
    pub fn is_active_phase(&self, time_ms: u64) -> bool {
        let t = (time_ms as f64 / self.period_ms as f64) * 2.0 * std::f64::consts::PI + self.phase;
        t.sin() > 0.0
    }
}

// ── StressResponse ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressResponse {
    pub stress_level: f64,
    pub threshold: f64,
}

impl StressResponse {
    pub fn new(threshold: f64) -> Self {
        Self {
            stress_level: 0.0,
            threshold,
        }
    }

    pub fn assess(&mut self, controller: &HomeostasisController) -> StressAction {
        let total_dev = controller.total_deviation();
        self.stress_level = total_dev;

        if total_dev > self.threshold * 3.0 {
            StressAction::Emergency
        } else if total_dev > self.threshold * 2.0 {
            StressAction::Conservative
        } else if total_dev > self.threshold {
            StressAction::Accelerated
        } else {
            StressAction::Normal
        }
    }

    pub fn recover(&mut self, rate: f64) {
        self.stress_level = (self.stress_level - rate).max(0.0);
    }
}
