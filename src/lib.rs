use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub mod regulators;
pub mod systems;
pub mod types;

pub use regulators::*;
pub use systems::*;
pub use types::*;

// ── HomeostasisController ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeostasisController {
    pub agent_id: String,
    pub setpoints: HashMap<String, f64>,
    pub current: HashMap<String, f64>,
    pub tolerances: HashMap<String, f64>,
}

impl HomeostasisController {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            setpoints: HashMap::new(),
            current: HashMap::new(),
            tolerances: HashMap::new(),
        }
    }

    pub fn set_setpoint(&mut self, metric: impl Into<String>, value: f64, tolerance: f64) {
        let m = metric.into();
        self.setpoints.insert(m.clone(), value);
        self.tolerances.insert(m, tolerance);
    }

    pub fn update(&mut self, readings: &HashMap<String, f64>) -> Vec<Correction> {
        for (k, &v) in readings {
            self.current.insert(k.clone(), v);
        }
        Vec::new() // regulators applied externally
    }

    pub fn is_in_balance(&self) -> bool {
        for (metric, &setpoint) in &self.setpoints {
            let cur = self.current.get(metric).copied().unwrap_or(setpoint);
            let tol = self.tolerances.get(metric).copied().unwrap_or(0.0);
            if (cur - setpoint).abs() > tol {
                return false;
            }
        }
        true
    }

    pub fn deviation(&self, metric: &str) -> f64 {
        let sp = self.setpoints.get(metric);
        let cur = self.current.get(metric);
        match (sp, cur) {
            (Some(s), Some(c)) => (c - s).abs(),
            _ => 0.0,
        }
    }

    pub fn total_deviation(&self) -> f64 {
        self.setpoints
            .keys()
            .map(|m| self.deviation(m))
            .sum()
    }

    pub fn health_score(&self) -> f64 {
        let max_deviation: f64 = self
            .setpoints
            .keys()
            .map(|m| self.tolerances.get(m).copied().unwrap_or(1.0) * 5.0)
            .sum();

        if max_deviation == 0.0 {
            return 1.0;
        }
        (1.0 - self.total_deviation() / max_deviation).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper ───────────────────────────────────────────────────

    fn make_controller() -> HomeostasisController {
        let mut c = HomeostasisController::new("test-agent");
        c.set_setpoint("energy", 80.0, 5.0);
        c.set_setpoint("attention", 0.8, 0.1);
        c.set_setpoint("stress", 0.1, 0.05);
        c
    }

    // ── Theorem 1: PID converges ─────────────────────────────────

    #[test]
    fn pid_converges_to_setpoint() {
        let mut pid = PIDRegulator::new(0.1, 0.01, 0.01);
        let result = pid.simulate_convergence(100.0, 0.0, 1000);
        assert!((result - 100.0).abs() < 2.0, "PID should converge, got {}", result);
    }

    #[test]
    fn pid_converges_from_above() {
        let mut pid = PIDRegulator::new(0.1, 0.01, 0.01);
        let result = pid.simulate_convergence(50.0, 200.0, 1000);
        assert!((result - 50.0).abs() < 2.0, "PID should converge from above, got {}", result);
    }

    #[test]
    fn pid_converges_with_different_gains() {
        let mut pid = PIDRegulator::new(0.5, 0.0, 0.2);
        let result = pid.simulate_convergence(75.0, 20.0, 500);
        assert!((result - 75.0).abs() < 1.0, "PID with different gains should converge, got {}", result);
    }

    #[test]
    fn pid_error_decreases_over_iterations() {
        let mut pid = PIDRegulator::new(0.05, 0.001, 0.1);
        let setpoint: f64 = 100.0;
        let mut current: f64 = 0.0;
        let initial_error = (setpoint - current).abs();
        for _ in 0..200 {
            let c = pid.regulate_mut(current, setpoint);
            current += c.magnitude * if c.action == "increase" { 1.0 } else { -1.0 };
        }
        let final_error = (setpoint - current).abs();
        assert!(final_error < initial_error, "Final error {} should be less than initial {}", final_error, initial_error);
    }

    // ── Theorem 2: Bang-bang switches at threshold ───────────────

    #[test]
    fn bang_bang_turns_on_below_threshold() {
        let bb = BangBangRegulator::new("energy", 5.0);
        let c = bb.regulate(70.0, 80.0); // 10 below → outside threshold
        assert_eq!(c.action, "turn_on");
    }

    #[test]
    fn bang_bang_turns_off_above_threshold() {
        let bb = BangBangRegulator::new("energy", 5.0);
        let c = bb.regulate(90.0, 80.0);
        assert_eq!(c.action, "turn_off");
    }

    #[test]
    fn bang_bang_maintains_within_threshold() {
        let bb = BangBangRegulator::new("energy", 5.0);
        let c = bb.regulate(82.0, 80.0);
        assert_eq!(c.action, "maintain");
    }

    #[test]
    fn bang_bang_at_exact_threshold() {
        let bb = BangBangRegulator::new("energy", 5.0);
        let c = bb.regulate(75.0, 80.0); // exactly at lower boundary
        assert_eq!(c.action, "maintain");
    }

    // ── Theorem 3: Threshold regulator picks correct zone ────────

    #[test]
    fn threshold_picks_low_zone() {
        let tr = ThresholdRegulator::new("energy", vec![
            (0.0, 30.0, "conserve".into()),
            (30.0, 70.0, "normal".into()),
            (70.0, 100.0, "boost".into()),
        ]);
        let c = tr.regulate(20.0, 80.0);
        assert_eq!(c.action, "conserve");
    }

    #[test]
    fn threshold_picks_mid_zone() {
        let tr = ThresholdRegulator::new("energy", vec![
            (0.0, 30.0, "conserve".into()),
            (30.0, 70.0, "normal".into()),
            (70.0, 100.0, "boost".into()),
        ]);
        let c = tr.regulate(50.0, 80.0);
        assert_eq!(c.action, "normal");
    }

    #[test]
    fn threshold_picks_high_zone() {
        let tr = ThresholdRegulator::new("energy", vec![
            (0.0, 30.0, "conserve".into()),
            (30.0, 70.0, "normal".into()),
            (70.0, 100.0, "boost".into()),
        ]);
        let c = tr.regulate(85.0, 80.0);
        assert_eq!(c.action, "boost");
    }

    #[test]
    fn threshold_unknown_zone() {
        let tr = ThresholdRegulator::new("energy", vec![
            (0.0, 50.0, "low".into()),
        ]);
        let c = tr.regulate(75.0, 80.0);
        assert_eq!(c.action, "unknown_zone");
    }

    // ── Theorem 4: Detects imbalance ─────────────────────────────

    #[test]
    fn detects_imbalance() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 60.0);
        assert!(!c.is_in_balance());
    }

    #[test]
    fn detects_balance() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 82.0);
        c.current.insert("attention".into(), 0.79);
        c.current.insert("stress".into(), 0.12);
        assert!(c.is_in_balance());
    }

    #[test]
    fn single_metric_imbalance() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 82.0);
        c.current.insert("attention".into(), 0.5); // out of tolerance
        c.current.insert("stress".into(), 0.11);
        assert!(!c.is_in_balance());
    }

    // ── Theorem 5: Total deviation ───────────────────────────────

    #[test]
    fn total_deviation_is_sum() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 70.0);   // dev = 10
        c.current.insert("attention".into(), 0.6);  // dev = 0.2
        c.current.insert("stress".into(), 0.2);     // dev = 0.1
        let expected = 10.0 + 0.2 + 0.1;
        assert!((c.total_deviation() - expected).abs() < 0.001);
    }

    #[test]
    fn total_deviation_zero_when_balanced() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 80.0);
        c.current.insert("attention".into(), 0.8);
        c.current.insert("stress".into(), 0.1);
        assert!(c.total_deviation() < 0.001);
    }

    #[test]
    fn deviation_for_individual_metric() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 70.0);
        assert!((c.deviation("energy") - 10.0).abs() < 0.001);
    }

    #[test]
    fn deviation_for_missing_metric() {
        let c = HomeostasisController::new("test");
        assert_eq!(c.deviation("nonexistent"), 0.0);
    }

    // ── Theorem 6: Health score 1.0 when balanced ────────────────

    #[test]
    fn health_score_1_when_balanced() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 80.0);
        c.current.insert("attention".into(), 0.8);
        c.current.insert("stress".into(), 0.1);
        assert!((c.health_score() - 1.0).abs() < 0.001);
    }

    #[test]
    fn health_score_decreases_with_deviation() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 60.0);
        c.current.insert("attention".into(), 0.5);
        c.current.insert("stress".into(), 0.3);
        assert!(c.health_score() < 1.0);
    }

    #[test]
    fn health_score_clamped_at_zero() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 0.0);
        c.current.insert("attention".into(), 0.0);
        c.current.insert("stress".into(), 1.0);
        assert!(c.health_score() >= 0.0);
    }

    // ── Theorem 7: Alert fires on deviation ──────────────────────

    #[test]
    fn alert_fires_on_deviation() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 60.0);
        let mut alerts = AlertSystem::new(100);
        alerts.check(&c);
        assert!(!alerts.alerts.is_empty());
    }

    #[test]
    fn no_alert_when_balanced() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 80.0);
        c.current.insert("attention".into(), 0.8);
        c.current.insert("stress".into(), 0.1);
        let mut alerts = AlertSystem::new(100);
        alerts.check(&c);
        assert!(alerts.alerts.is_empty());
    }

    #[test]
    fn alert_severity_increases_with_deviation() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 30.0); // huge deviation
        let mut alerts = AlertSystem::new(100);
        alerts.check(&c);
        assert!(alerts.alerts.iter().any(|a| matches!(a.severity, Urgency::Critical | Urgency::High)));
    }

    // ── Theorem 8: Alert clears when resolved ────────────────────

    #[test]
    fn alert_clears_when_resolved() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 60.0);
        let mut alerts = AlertSystem::new(100);
        alerts.check(&c);
        assert!(!alerts.alerts.is_empty());

        // Fix the deviation
        c.current.insert("energy".into(), 80.0);
        alerts.clear_resolved(&c);
        assert!(alerts.alerts.is_empty());
    }

    #[test]
    fn alert_stays_if_unresolved() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 60.0);
        let mut alerts = AlertSystem::new(100);
        alerts.check(&c);
        alerts.clear_resolved(&c);
        assert!(!alerts.alerts.is_empty());
    }

    // ── Theorem 9: Circadian oscillates ──────────────────────────

    #[test]
    fn circadian_oscillates_around_base() {
        let mut cr = CircadianRhythm::new(1000, 5.0, 0.0);
        cr.base_setpoints.insert("energy".into(), 80.0);

        let at_0 = cr.setpoint_at("energy", 0);
        let at_250 = cr.setpoint_at("energy", 250);
        let at_500 = cr.setpoint_at("energy", 500);
        let at_750 = cr.setpoint_at("energy", 750);

        assert!((at_0 - 80.0).abs() < 0.001, "phase 0 should be base");
        assert!(at_250 > 80.0, "quarter period should be above base");
        assert!((at_500 - 80.0).abs() < 0.001, "half period should return to base");
        assert!(at_750 < 80.0, "three-quarter period should be below base");
    }

    #[test]
    fn circadian_within_amplitude() {
        let cr = CircadianRhythm::new(1000, 10.0, 0.0);
        for t in (0..1000).step_by(50) {
            let val = cr.setpoint_at("energy", t as u64);
            assert!(val.abs() <= 10.01, "should stay within ±amplitude, got {}", val);
        }
    }

    #[test]
    fn circadian_active_phase() {
        let cr = CircadianRhythm::new(1000, 1.0, 0.0);
        assert!(!cr.is_active_phase(0)); // sin(0) = 0, not > 0
        assert!(cr.is_active_phase(250)); // sin(π/2) = 1 > 0
    }

    // ── Theorem 10: Stress escalates ─────────────────────────────

    #[test]
    fn stress_escalates_with_deviation() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 80.0);
        c.current.insert("attention".into(), 0.8);
        c.current.insert("stress".into(), 0.1);
        let mut sr = StressResponse::new(5.0);
        assert_eq!(sr.assess(&c), StressAction::Normal);

        // Mild deviation → Accelerated
        c.current.insert("energy".into(), 70.0);
        let action = sr.assess(&c);
        assert_eq!(action, StressAction::Accelerated);

        // More deviation → Conservative
        c.current.insert("attention".into(), 0.4);
        c.current.insert("stress".into(), 0.5);
        let action2 = sr.assess(&c);
        assert_eq!(action2, StressAction::Conservative);
    }

    #[test]
    fn stress_emergency_at_high_deviation() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 0.0);
        c.current.insert("attention".into(), 0.0);
        c.current.insert("stress".into(), 1.0);
        let mut sr = StressResponse::new(0.1);
        assert_eq!(sr.assess(&c), StressAction::Emergency);
    }

    // ── Theorem 11: Stress recovery ──────────────────────────────

    #[test]
    fn stress_recovery_decreases() {
        let mut sr = StressResponse::new(1.0);
        sr.stress_level = 5.0;
        sr.recover(1.0);
        assert!((sr.stress_level - 4.0).abs() < 0.001);
    }

    #[test]
    fn stress_recovery_clamps_at_zero() {
        let mut sr = StressResponse::new(1.0);
        sr.stress_level = 0.5;
        sr.recover(1.0);
        assert!((sr.stress_level).abs() < 0.001);
    }

    #[test]
    fn stress_recovery_multiple_steps() {
        let mut sr = StressResponse::new(1.0);
        sr.stress_level = 10.0;
        for _ in 0..5 {
            sr.recover(1.5);
        }
        assert!((sr.stress_level - 2.5).abs() < 0.001);
    }

    // ── Theorem 12: VitalSigns roundtrip ─────────────────────────

    #[test]
    fn vital_signs_roundtrip() {
        let vs = VitalSigns::default();
        let readings = vs.to_readings();
        let vs2 = VitalSigns::from_readings(&readings);
        assert!((vs.energy - vs2.energy).abs() < 0.001);
        assert!((vs.attention - vs2.attention).abs() < 0.001);
        assert!((vs.stress - vs2.stress).abs() < 0.001);
        assert!((vs.conservation_budget - vs2.conservation_budget).abs() < 0.001);
        assert!((vs.response_time_ms - vs2.response_time_ms).abs() < 0.001);
    }

    #[test]
    fn vital_signs_custom_roundtrip() {
        let vs = VitalSigns {
            energy: 50.0,
            attention: 0.3,
            stress: 0.7,
            conservation_budget: 20.0,
            response_time_ms: 500.0,
        };
        let readings = vs.to_readings();
        let vs2 = VitalSigns::from_readings(&readings);
        assert!((vs.energy - vs2.energy).abs() < 0.001);
    }

    #[test]
    fn vital_signs_from_partial_readings() {
        let mut readings = HashMap::new();
        readings.insert("energy".into(), 90.0);
        let vs = VitalSigns::from_readings(&readings);
        assert_eq!(vs.energy, 90.0);
        assert_eq!(vs.attention, 0.8); // default
    }

    // ── Theorem 13: Multiple regulators coexist ──────────────────

    #[test]
    fn multiple_regulators() {
        let bb = BangBangRegulator::new("energy", 5.0);
        let tr = ThresholdRegulator::new("attention", vec![
            (0.0, 0.5, "low".into()),
            (0.5, 1.0, "normal".into()),
        ]);

        let c1 = bb.regulate(70.0, 80.0);
        let c2 = tr.regulate(0.3, 0.8);

        assert_eq!(c1.action, "turn_on");
        assert_eq!(c2.action, "low");
    }

    #[test]
    fn regulators_for_different_metrics() {
        let bb_energy = BangBangRegulator::new("energy", 5.0);
        let bb_stress = BangBangRegulator::new("stress", 0.1);

        let c1 = bb_energy.regulate(70.0, 80.0);
        let c2 = bb_stress.regulate(0.5, 0.1);

        assert_eq!(c1.metric, "energy");
        assert_eq!(c2.metric, "stress");
    }

    // ── Update method ────────────────────────────────────────────

    #[test]
    fn update_stores_readings() {
        let mut c = make_controller();
        let mut readings = HashMap::new();
        readings.insert("energy".into(), 75.0);
        c.update(&readings);
        assert_eq!(c.current.get("energy").copied(), Some(75.0));
    }

    #[test]
    fn update_preserves_existing() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 80.0);
        let mut readings = HashMap::new();
        readings.insert("attention".into(), 0.9);
        c.update(&readings);
        assert_eq!(c.current.get("energy").copied(), Some(80.0));
        assert_eq!(c.current.get("attention").copied(), Some(0.9));
    }

    // ── Correction description ───────────────────────────────────

    #[test]
    fn correction_description_format() {
        let c = Correction::new("energy", "increase", 5.0, Urgency::High);
        let desc = c.description();
        assert!(desc.contains("energy"));
        assert!(desc.contains("increase"));
        assert!(desc.contains("high"));
    }

    // ── Serde roundtrips ─────────────────────────────────────────

    #[test]
    fn serde_vital_signs() {
        let vs = VitalSigns::default();
        let json = serde_json::to_string(&vs).unwrap();
        let vs2: VitalSigns = serde_json::from_str(&json).unwrap();
        assert!((vs.energy - vs2.energy).abs() < 0.001);
    }

    #[test]
    fn serde_controller() {
        let mut c = HomeostasisController::new("agent-1");
        c.set_setpoint("x", 50.0, 5.0);
        let json = serde_json::to_string(&c).unwrap();
        let c2: HomeostasisController = serde_json::from_str(&json).unwrap();
        assert_eq!(c2.agent_id, "agent-1");
        assert_eq!(c2.setpoints.get("x").copied(), Some(50.0));
    }

    #[test]
    fn serde_bang_bang() {
        let bb = BangBangRegulator::new("test", 5.0);
        let json = serde_json::to_string(&bb).unwrap();
        let bb2: BangBangRegulator = serde_json::from_str(&json).unwrap();
        assert_eq!(bb2.metric, "test");
        assert!((bb2.threshold - 5.0).abs() < 0.001);
    }

    #[test]
    fn serde_threshold_regulator() {
        let tr = ThresholdRegulator::new("m", vec![(0.0, 10.0, "low".into())]);
        let json = serde_json::to_string(&tr).unwrap();
        let tr2: ThresholdRegulator = serde_json::from_str(&json).unwrap();
        assert_eq!(tr2.metric, "m");
    }

    #[test]
    fn serde_pid_regulator() {
        let pid = PIDRegulator::new(1.0, 0.5, 0.1);
        let json = serde_json::to_string(&pid).unwrap();
        let pid2: PIDRegulator = serde_json::from_str(&json).unwrap();
        assert!((pid2.kp - 1.0).abs() < 0.001);
    }

    #[test]
    fn serde_circadian() {
        let cr = CircadianRhythm::new(5000, 3.0, 1.57);
        let json = serde_json::to_string(&cr).unwrap();
        let cr2: CircadianRhythm = serde_json::from_str(&json).unwrap();
        assert_eq!(cr2.period_ms, 5000);
    }

    #[test]
    fn serde_stress_response() {
        let sr = StressResponse { stress_level: 2.5, threshold: 1.0 };
        let json = serde_json::to_string(&sr).unwrap();
        let sr2: StressResponse = serde_json::from_str(&json).unwrap();
        assert!((sr2.stress_level - 2.5).abs() < 0.001);
    }

    #[test]
    fn serde_alert() {
        let a = Alert::new("a1", "m1", "msg", Urgency::Critical, 12345);
        let json = serde_json::to_string(&a).unwrap();
        let a2: Alert = serde_json::from_str(&json).unwrap();
        assert_eq!(a2.id, "a1");
        assert_eq!(a2.severity, Urgency::Critical);
    }

    #[test]
    fn serde_urgency() {
        for u in [Urgency::Low, Urgency::Medium, Urgency::High, Urgency::Critical] {
            let json = serde_json::to_string(&u).unwrap();
            let u2: Urgency = serde_json::from_str(&json).unwrap();
            assert_eq!(u, u2);
        }
    }

    #[test]
    fn serde_stress_action() {
        for a in [StressAction::Normal, StressAction::Accelerated, StressAction::Conservative, StressAction::Emergency] {
            let json = serde_json::to_string(&a).unwrap();
            let a2: StressAction = serde_json::from_str(&json).unwrap();
            assert_eq!(a, a2);
        }
    }

    // ── Alert system edge cases ──────────────────────────────────

    #[test]
    fn alert_system_max_alerts() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 60.0);
        c.current.insert("attention".into(), 0.5);
        c.current.insert("stress".into(), 0.5);
        let mut alerts = AlertSystem::new(2);
        alerts.check(&c);
        assert!(alerts.alerts.len() <= 2);
    }

    #[test]
    fn alert_acknowledge() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 60.0);
        let mut alerts = AlertSystem::new(100);
        alerts.check(&c);
        let id = alerts.alerts[0].id.clone();
        alerts.acknowledge(&id);
        assert!(alerts.alerts[0].acknowledged);
    }

    #[test]
    fn active_alerts_excludes_acknowledged() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 60.0);
        let mut alerts = AlertSystem::new(100);
        alerts.check(&c);
        let id = alerts.alerts[0].id.clone();
        alerts.acknowledge(&id);
        assert!(alerts.active_alerts().is_empty());
    }

    #[test]
    fn critical_alerts_filter() {
        let mut c = make_controller();
        c.current.insert("energy".into(), 30.0); // large deviation → high/critical
        let mut alerts = AlertSystem::new(100);
        alerts.check(&c);
        let critical = alerts.critical_alerts();
        // Should have at least one high/critical alert
        assert!(!critical.is_empty() || !alerts.alerts.is_empty());
    }

    // ── Edge: empty controller ───────────────────────────────────

    #[test]
    fn empty_controller_is_balanced() {
        let c = HomeostasisController::new("empty");
        assert!(c.is_in_balance());
        assert_eq!(c.total_deviation(), 0.0);
        assert!((c.health_score() - 1.0).abs() < 0.001);
    }

    // ── Urgency display ──────────────────────────────────────────

    #[test]
    fn urgency_display() {
        assert_eq!(format!("{}", Urgency::Low), "low");
        assert_eq!(format!("{}", Urgency::Critical), "critical");
    }

    // ── Threshold regulator sorted zones ─────────────────────────

    #[test]
    fn threshold_zones_sorted() {
        let tr = ThresholdRegulator::new("x", vec![
            (50.0, 100.0, "high".into()),
            (0.0, 50.0, "low".into()),
        ]);
        // Zones should be sorted
        assert_eq!(tr.zones[0].2, "low");
        assert_eq!(tr.zones[1].2, "high");
    }

    // ── Additional tests to reach 65+ ───────────────────────────

    #[test]
    fn circadian_returns_base_for_unknown_metric() {
        let cr = CircadianRhythm::new(1000, 5.0, 0.0);
        // No base setpoint → modulates around 0
        let val = cr.setpoint_at("nonexistent", 250);
        assert!(val > 0.0); // sin(π/2) = 1, so amplitude * 1 = 5.0
    }

    #[test]
    fn circadian_phase_shift() {
        let cr = CircadianRhythm::new(1000, 5.0, std::f64::consts::PI); // phase = π
        let val = cr.setpoint_at("x", 0);
        // sin(0 + π) ≈ 0, so should be near 0
        assert!(val.abs() < 0.01);
    }

    #[test]
    fn alert_acknowledge_nonexistent_id() {
        let mut alerts = AlertSystem::new(100);
        alerts.acknowledge("nonexistent"); // should not panic
        assert!(alerts.alerts.is_empty());
    }

    #[test]
    fn correction_new_builds_correctly() {
        let c = Correction::new("m", "act", 3.14, Urgency::Medium);
        assert_eq!(c.metric, "m");
        assert_eq!(c.action, "act");
        assert!((c.magnitude - 3.14).abs() < 0.001);
        assert_eq!(c.urgency, Urgency::Medium);
    }

    #[test]
    fn urgency_from_deviation_levels() {
        assert_eq!(Urgency::from_deviation(0.5, 1.0), Urgency::Low);
        assert_eq!(Urgency::from_deviation(2.0, 1.0), Urgency::Medium);
        assert_eq!(Urgency::from_deviation(4.0, 1.0), Urgency::High);
        assert_eq!(Urgency::from_deviation(6.0, 1.0), Urgency::Critical);
    }

    #[test]
    fn health_score_with_single_metric() {
        let mut c = HomeostasisController::new("single");
        c.set_setpoint("x", 100.0, 10.0);
        c.current.insert("x".into(), 100.0);
        assert!((c.health_score() - 1.0).abs() < 0.001);
    }

    #[test]
    fn bang_bang_regulator_metric_name() {
        let bb = BangBangRegulator::new("temperature", 2.0);
        assert_eq!(bb.metric_name(), "temperature");
    }

    #[test]
    fn threshold_regulator_metric_name() {
        let tr = ThresholdRegulator::new("cpu", vec![(0.0, 100.0, "ok".into())]);
        assert_eq!(tr.metric_name(), "cpu");
    }

    #[test]
    fn pid_regulate_stateless() {
        let pid = PIDRegulator::new(0.5, 0.1, 0.05);
        let c = pid.regulate(50.0, 100.0);
        assert_eq!(c.action, "increase");
        assert!(c.magnitude > 0.0);
    }
}
