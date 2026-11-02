//! The processor time a thread has used (`xmip_core::processor_time`),
//! held to the clock beside it.

use std::time::{Duration, Instant};

use xmip_core::processor_time::ProcessorTime;

#[test]
fn a_spin_is_counted_and_a_sleep_is_not() {
    let spun = ProcessorTime::thread();
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(20) {
        std::hint::spin_loop();
    }
    let spent = spun.elapsed();
    let wall = started.elapsed();
    // Never more than the wall clock; less where the spin was preempted.
    assert!(
        spent > Duration::ZERO && spent <= wall + Duration::from_millis(1),
        "a spin of {wall:?} counted {spent:?}"
    );

    let slept = ProcessorTime::thread();
    std::thread::sleep(Duration::from_millis(50));
    let waited = slept.elapsed();
    assert!(
        waited < Duration::from_millis(5),
        "a sleep of 50 ms counted {waited:?}"
    );
}
