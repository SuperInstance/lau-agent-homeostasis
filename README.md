# lau-agent-homeostasis

**Self-regulating mechanisms that keep an agent alive and balanced.** A library for building agents that maintain internal stability through control theory — PID controllers, bang-bang regulators, threshold zones, circadian rhythms, stress responses, and alert systems.

Biological homeostasis for software agents: every metric has a setpoint and tolerance, regulators produce corrections when things drift, and the system escalates from "Normal" to "Emergency" as deviations grow.

## What This Does

- **HomeostasisController** — The central hub. Holds setpoints, current readings, and tolerances for named metrics (energy, attention, stress, etc.). Detects imbalance, computes deviation, and produces a health score.
- **PIDRegulator** — A classic proportional-integral-derivative controller. Tracks error history and produces smooth, converging corrections. Includes a simulation mode for testing convergence.
- **BangBangRegulator** — On/off control: activate below a lower threshold, deactivate above an upper threshold, maintain in between. Simple but effective.
- **ThresholdRegulator** — Maps current values to named zones (e.g., 0–30 = "conserve", 30–70 = "normal", 70–100 = "boost"). Produces zone-appropriate actions.
- **CircadianRhythm** — Sinusoidal modulation of setpoints over time. Simulates biological cycles where optimal values shift predictably.
- **StressResponse** — Escalating response to cumulative deviation. Four levels: Normal → Accelerated → Conservative → Emergency.
- **AlertSystem** — Monitors a controller and generates timestamped alerts when metrics leave tolerance. Supports acknowledgement, filtering, and auto-clearing when deviations resolve.
- **VitalSigns** — A structured set of five key metrics (energy, attention, stress, conservation_budget, response_time_ms) with serialization support.

## Key Idea

Biological organisms maintain homeostasis: body temperature, blood sugar, heart rate — all regulated around setpoints. This library gives software agents the same capability.

The core loop:
1. **Sense** → Read current metric values
2. **Compare** → Check against setpoints + tolerances
3. **Correct** → Generate a `Correction` (action + magnitude + urgency)
4. **Act** → Apply the correction
5. **Monitor** → Alert system watches for persistent deviations

Multiple regulators can coexist for different metrics, each using the strategy best suited to that metric's dynamics. A PID controller for smooth regulation (energy), bang-bang for binary decisions (on/off), threshold for categorical responses (conserve/normal/boost), and circadian for time-varying targets.

## Install

```toml
[dependencies]
lau-agent-homeostasis = "0.1.0"
```

## Quick Start

### Basic homeostasis monitoring

```rust
use lau_agent_homeostasis::*;

// Set up a controller with target metrics
let mut ctrl = HomeostasisController::new("agent-1");
ctrl.set_setpoint("energy", 80.0, 5.0);     // target 80 ± 5
ctrl.set_setpoint("attention", 0.8, 0.1);   // target 0.8 ± 0.1
ctrl.set_setpoint("stress", 0.1, 0.05);     // target 0.1 ± 0.05

// Feed current readings
let mut readings = std::collections::HashMap::new();
readings.insert("energy".into(), 65.0);
readings.insert("attention".into(), 0.7);
readings.insert("stress".into(), 0.15);
ctrl.update(&readings);

// Check health
println!("Balanced: {}", ctrl.is_in_balance());     // false
println!("Health: {:.1}%", ctrl.health_score() * 100.0);
println!("Energy deviation: {:.1}", ctrl.deviation("energy"));
```

### PID regulation with convergence

```rust
let mut pid = PIDRegulator::new(0.1, 0.01, 0.01);
let final_value = pid.simulate_convergence(100.0, 0.0, 1000);
println!("Converged to: {:.2}", final_value); // ≈ 100.0
```

### Bang-bang control

```rust
let bb = BangBangRegulator::new("heater", 5.0);
let c1 = bb.regulate(70.0, 80.0); // below range → turn_on
let c2 = bb.regulate(90.0, 80.0); // above range → turn_off
let c3 = bb.regulate(82.0, 80.0); // within range → maintain
```

### Alert system

```rust
let mut alerts = AlertSystem::new(100);
alerts.check(&ctrl);              // generates alerts for deviating metrics
println!("Active: {:?}", alerts.active_alerts());
println!("Critical: {:?}", alerts.critical_alerts());
alerts.acknowledge(&alerts.alerts[0].id);

// When metrics return to normal:
alerts.clear_resolved(&ctrl);
```

### Circadian rhythm

```rust
let mut cr = CircadianRhythm::new(86_400_000, 10.0, 0.0); // 24h period
cr.base_setpoints.insert("energy".into(), 80.0);

let morning = cr.setpoint_at("energy", 21_600_000);  // 6h — high energy
let night = cr.setpoint_at("energy", 64_800_000);    // 18h — lower energy
println!("Morning target: {:.1}, Night target: {:.1}", morning, night);
```

### Stress response

```rust
let mut stress = StressResponse::new(5.0);
match stress.assess(&ctrl) {
    StressAction::Normal => println!("All good"),
    StressAction::Accelerated => println!("Speeding up corrections"),
    StressAction::Conservative => println!("Reducing non-essential activity"),
    StressAction::Emergency => println!("Critical survival mode"),
}

// Recovery when things improve:
stress.recover(1.0); // decreases stress level by 1.0
```

## API Reference

### Types

#### `Urgency`

```rust
enum Urgency { Low, Medium, High, Critical }
```

Classifies severity. `from_deviation(deviation, tolerance)` maps deviation/tolerance ratio to urgency levels (< 1.5× = Low, < 3× = Medium, < 5× = High, ≥ 5× = Critical).

#### `Correction`

```rust
struct Correction {
    metric: String,
    action: String,       // "increase", "decrease", "turn_on", "maintain", etc.
    magnitude: f64,
    urgency: Urgency,
}
```

#### `Alert`

```rust
struct Alert {
    id: String,
    metric: String,
    message: String,
    severity: Urgency,
    timestamp: u64,
    acknowledged: bool,
}
```

#### `VitalSigns`

Five core metrics with defaults and HashMap conversion.

| Field | Default |
|---|---|
| `energy` | 80.0 |
| `attention` | 0.8 |
| `stress` | 0.1 |
| `conservation_budget` | 50.0 |
| `response_time_ms` | 100.0 |

#### `StressAction`

```rust
enum StressAction { Normal, Accelerated, Conservative, Emergency }
```

### Regulators

All regulators implement the `Regulator` trait:

```rust
trait Regulator: Send + Sync {
    fn metric_name(&self) -> &str;
    fn regulate(&self, current: f64, setpoint: f64) -> Correction;
}
```

#### `PIDRegulator`

| Method | Description |
|---|---|
| `new(kp, ki, kd)` | Create with PID gains |
| `regulate(current, setpoint)` | Stateless P-only correction |
| `regulate_mut(current, setpoint)` | Stateful PID with integral/derivative tracking |
| `simulate_convergence(setpoint, initial, steps)` | Run N steps, return final value |

#### `BangBangRegulator`

| Method | Description |
|---|---|
| `new(metric, threshold)` | Create with symmetric threshold around setpoint |
| `regulate(current, setpoint)` | "turn_on" / "turn_off" / "maintain" |

#### `ThresholdRegulator`

| Method | Description |
|---|---|
| `new(metric, zones)` | Create with (low, high, action_name) zones (auto-sorted) |
| `regulate(current, _)` | Returns matching zone's action, or "unknown_zone" |

### Systems

#### `HomeostasisController`

| Method | Description |
|---|---|
| `new(agent_id)` | Create empty controller |
| `set_setpoint(metric, value, tolerance)` | Register a target |
| `update(&readings)` | Feed current sensor values |
| `is_in_balance()` | All metrics within tolerance |
| `deviation(metric)` | Absolute deviation for one metric |
| `total_deviation()` | Sum of all deviations |
| `health_score()` | 0.0–1.0, clamped, based on deviation vs max |

#### `AlertSystem`

| Method | Description |
|---|---|
| `new(max_alerts)` | Create with capacity |
| `check(&controller)` | Scan for deviations, generate alerts |
| `active_alerts()` | Unacknowledged alerts |
| `critical_alerts()` | High + Critical severity only |
| `acknowledge(id)` | Mark alert as seen |
| `clear_resolved(&controller)` | Remove alerts for metrics back in range |

#### `CircadianRhythm`

| Method | Description |
|---|---|
| `new(period_ms, amplitude, phase)` | Create sinusoidal modulator |
| `setpoint_at(metric, time_ms)` | Base + amplitude × sin(2πt/T + φ) |
| `is_active_phase(time_ms)` | True when sin > 0 |

#### `StressResponse`

| Method | Description |
|---|---|
| `new(threshold)` | Create with base threshold |
| `assess(&controller)` | Classify stress level into `StressAction` |
| `recover(rate)` | Decrease stress level |

## How It Works

### PID Controller

The PID regulator computes:

```
output = Kp × error + Ki × Σ(error) + Kd × Δerror
```

- **P (proportional):** Correction proportional to current error. Fast response, may oscillate.
- **I (integral):** Accumulates past error. Eliminates steady-state offset but can cause windup.
- **D (derivative):** Responds to rate of change. Dampens oscillation.

The mutable `regulate_mut` tracks integral and previous error across calls. The `simulate_convergence` method runs the full closed-loop: sense → correct → apply → repeat, with integral windup clamped to ±1000.

### Bang-Bang Control

The simplest control strategy: two states (on/off) with a deadband. If current < setpoint − threshold, activate. If current > setpoint + threshold, deactivate. Otherwise, maintain. Produces fast response but oscillation around the setpoint.

### Threshold Zones

Maps a continuous value to discrete categories. Zones are sorted by lower bound at construction time. The regulator finds the first zone containing the current value and returns its named action. Values outside all zones get "unknown_zone" with Critical urgency.

### Circadian Modulation

Setpoints aren't always constant. The `CircadianRhythm` modulates base values sinusoidally:

```
setpoint(t) = base + amplitude × sin(2π × t / period + phase)
```

The `is_active_phase` check (sin > 0) divides the cycle into active and rest periods, modeling biological day/night cycles.

### Stress Escalation

The `StressResponse` uses the total deviation across all metrics, scaled by a threshold:

| Total Deviation | Action |
|---|---|
| ≤ threshold × 1 | Normal |
| ≤ threshold × 2 | Accelerated |
| ≤ threshold × 3 | Conservative |
| > threshold × 3 | Emergency |

Recovery decreases the accumulated stress level at a configurable rate, modeling the return to calm after disturbances resolve.

### Health Score

```
health = max(0, 1 − total_deviation / max_deviation)
```

Where `max_deviation = Σ(5 × tolerance)` for all metrics. A perfectly balanced agent scores 1.0. The score clamps at 0.0 — no negative health.

### Alert Lifecycle

1. **Generation:** `check()` scans all metrics. If deviation > tolerance, an alert is created with urgency based on the deviation ratio.
2. **Capacity:** If alerts exceed `max_alerts`, oldest are dropped (FIFO).
3. **Acknowledgement:** `acknowledge()` marks alerts as seen. Acknowledged alerts don't appear in `active_alerts()`.
4. **Resolution:** `clear_resolved()` removes alerts whose metrics have returned within tolerance.

## The Math

### PID Control

The standard PID control law in discrete time:

```
u(t) = Kp × e(t) + Ki × Σᵢ₌₀ᵗ e(i) + Kd × (e(t) − e(t−1))
```

where `e(t) = setpoint − current` is the error signal. For a first-order system (which this library simulates), the closed-loop transfer function has characteristic equation:

```
s² + (Kd/Kp)s + (Ki/Kp) = 0
```

Properly tuned PID gains ensure the roots have negative real parts → exponential convergence to setpoint.

### Bang-Bang Oscillation

A bang-bang controller with threshold δ and setpoint r switches at r ± δ. For a system with dynamics `dx/dt = −αx + u` and control `u ∈ {0, U}`:

The state oscillates between r − δ and r + δ with period proportional to `δ / (α × U)`. Smaller threshold → faster switching → more wear. Larger threshold → slower switching → wider oscillation.

### Circadian Rhythm

The modulation follows a sinusoid:

```
s(t) = A × sin(2πt/T + φ) + B
```

where A = amplitude, T = period, φ = phase offset, B = base setpoint. The Fourier spectrum is a single spike at frequency 1/T, making this the simplest periodic signal.

### Urgency Classification

Urgency is based on the ratio `r = deviation / tolerance`:

```
r < 1.5  → Low       (within ~50% of tolerance)
r < 3.0  → Medium    (1–3× tolerance)
r < 5.0  → High      (3–5× tolerance)
r ≥ 5.0  → Critical  (severe deviation)
```

These thresholds create a piecewise-linear mapping from continuous deviation to discrete urgency, enabling appropriate escalation of response.

## Testing

69 tests covering:

- PID convergence from above and below, error decrease, different gain settings
- Bang-bang switching at thresholds, maintaining within deadband
- Threshold zone selection, unknown zone handling, zone sorting
- Imbalance detection, single-metric and multi-metric
- Total deviation computation, individual deviation, missing metrics
- Health score at balance, with deviation, clamped at zero
- Alert generation, severity escalation, acknowledgement, clearing, capacity limits
- Circadian oscillation, amplitude bounds, phase shift, active phase detection
- Stress escalation through all four levels, recovery, multi-step recovery
- VitalSigns roundtrip, partial readings
- Multiple regulators coexisting
- Serde round-trips for all serializable types
- Edge cases: empty controller, nonexistent metrics, urgency display formatting

Run with `cargo test`.

## License

MIT
