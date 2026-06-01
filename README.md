# lau-agent-homeostasis

> Agent homeostasis — self-regulating mechanisms that keep an agent alive and balanced

## What This Does

Agent homeostasis — self-regulating mechanisms that keep an agent alive and balanced. Part of the PLATO/LAU ecosystem — a mathematically rigorous framework for building educational agents that learn, teach, and evolve.

## The Key Idea

This crate implements the core abstractions needed for its domain, with a focus on correctness, composability, and conservation guarantees. Every public type is serializable (serde), every algorithm is tested, and every invariant is verified.

## Install

```bash
cargo add lau-agent-homeostasis
```

## Quick Start

See the API Reference below for complete usage. Key entry points:

```rust
use lau_agent_homeostasis::*;
// See types and methods below for complete usage
```

## API Reference

```rust
pub struct AlertSystem 
    pub fn new(max_alerts: usize) -> Self 
    pub fn check(&mut self, controller: &HomeostasisController) 
    pub fn active_alerts(&self) -> Vec<&Alert> 
    pub fn critical_alerts(&self) -> Vec<&Alert> 
    pub fn acknowledge(&mut self, id: &str) 
    pub fn clear_resolved(&mut self, controller: &HomeostasisController) 
pub struct CircadianRhythm 
    pub fn new(period_ms: u64, amplitude: f64, phase: f64) -> Self 
    pub fn setpoint_at(&self, metric: &str, time_ms: u64) -> f64 
    pub fn is_active_phase(&self, time_ms: u64) -> bool 
pub struct StressResponse 
    pub fn new(threshold: f64) -> Self 
    pub fn assess(&mut self, controller: &HomeostasisController) -> StressAction 
    pub fn recover(&mut self, rate: f64) 
pub enum Urgency 
    pub fn from_deviation(deviation: f64, tolerance: f64) -> Self 
pub struct Correction 
    pub fn new(metric: impl Into<String>, action: impl Into<String>, magnitude: f64, urgency: Urgency) -> Self 
    pub fn description(&self) -> String 
pub struct Alert 
    pub fn new(id: impl Into<String>, metric: impl Into<String>, message: impl Into<String>, severity: Urgency, timestamp: u64) -> Self 
pub struct VitalSigns 
    pub fn to_readings(&self) -> HashMap<String, f64> 
    pub fn from_readings(readings: &HashMap<String, f64>) -> Self 
pub enum StressAction 
pub trait Regulator: Send + Sync 
pub struct PIDRegulator 
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self 
    pub fn regulate_mut(&mut self, current: f64, setpoint: f64) -> Correction 
    pub fn simulate_convergence(&mut self, setpoint: f64, initial: f64, steps: usize) -> f64 
pub struct BangBangRegulator 
    pub fn new(metric: impl Into<String>, threshold: f64) -> Self 
pub struct ThresholdRegulator 
    pub fn new(metric: impl Into<String>, zones: Vec<(f64, f64, String)>) -> Self 
pub struct HomeostasisController 
    pub fn new(agent_id: impl Into<String>) -> Self 
    pub fn set_setpoint(&mut self, metric: impl Into<String>, value: f64, tolerance: f64) 
    pub fn update(&mut self, readings: &HashMap<String, f64>) -> Vec<Correction> 
    pub fn is_in_balance(&self) -> bool 
    pub fn deviation(&self, metric: &str) -> f64 
    pub fn total_deviation(&self) -> f64 
    pub fn health_score(&self) -> f64 
```

## How It Works

Read the source in `src/` for full implementation details. All algorithms are documented with inline comments explaining the mathematical foundations.

## The Math

This crate implements formal mathematical constructs. See the source documentation for theorem statements and proofs of correctness.

## Testing

**69 tests** covering construction, serialization, correctness properties, edge cases, and composability with other lau-* crates.

## License

MIT
