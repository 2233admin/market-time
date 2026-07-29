//! Mark Time — thin CLI shell.
//!
//! Holds no domain logic. Reads the clock (the core must not), loads rule data via
//! `market-time-data` (the core cannot), and renders what the core returns.
//!
//! Where this shell reports "now" it MUST also surface the host's clock discipline
//! bounds as uncertainty rather than presenting the host clock as exact — constitution,
//! Domain and Data Constraints. Wide-area internet time synchronisation does not reach
//! nanoseconds, and no surface here may imply that it does.

fn main() {
    // Scaffold only. Commands land with T038/T039 per tasks.md.
    println!("market-time: not implemented yet");
}
