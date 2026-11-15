//! A thread waiting under a lock until woken or due (`xmip_core::wake`),
//! held to the millisecond.

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use xmip_core::latency::{Rounds, spin, spread};
use xmip_core::lock::Locked;
use xmip_core::processor_time::ProcessorTime;
use xmip_core::wake::Wake;

/// How many waits each test measures.
const ROUNDS: usize = 200;

/// A due time is kept within a millisecond, apart from load: held to
/// `std::thread::sleep` measured beside it, which on Windows already waits
/// on a high-resolution timer, the fastest of five
/// (`xmip_core::latency::spread`). Until 2026-10-16 a timed wait ended on
/// the system tick, up to 9-14 ms late.
#[test]
fn a_wait_ends_within_a_millisecond_of_its_due_time() {
    const DUE: Duration = Duration::from_millis(2);
    let wake = Wake::new();
    let mutex = Mutex::new(());
    let (mut late, mut control) = (Vec::new(), Vec::new());
    for _ in 0..ROUNDS {
        let began = Instant::now();
        let (guard, came) = wake.wait_timeout(&mutex, mutex.locked(), DUE);
        late.push(began.elapsed().saturating_sub(DUE));
        assert!(came, "nothing notified");
        drop(guard);
        let began = Instant::now();
        thread::sleep(DUE);
        control.push(began.elapsed().saturating_sub(DUE));
    }
    let late = spread("a 2 ms wait late", late);
    let control = spread("thread::sleep beside it, late", control);
    // The fastest of five to the millisecond; the tail, which load alone
    // moves by milliseconds either way, within the worst the machine did.
    assert!(
        late.fastest_of_five < Duration::from_millis(1) + control.fastest_of_five,
        "the fastest of five {:?} late, the machine's own {:?}",
        late.fastest_of_five,
        control.fastest_of_five
    );
    assert!(
        late.p99 < Duration::from_millis(1) + control.worst,
        "p99 {:?} late, the machine's worst {:?}",
        late.p99,
        control.worst
    );
}

/// A notify wakes a waiting thread within a millisecond, apart from load,
/// and says it was not the due time: from the notify to the waiter
/// running, round by round beside a plain wake (`xmip_core::latency::Rounds`;
/// a notify wakes one thread, as a plain wake does), the waiter's answer
/// back on a channel in the round's processor time, as a plain wake's is.
/// Until 2026-11-15 its 99th percentile was held under five milliseconds.
#[test]
fn a_notify_wakes_within_a_millisecond() {
    let shared = Arc::new((Mutex::new(None::<Instant>), Wake::new()));
    let (answer, answered) = mpsc::channel();
    let waiter = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            let (mutex, wake) = &*shared;
            let mut guard = mutex.locked();
            for _ in 0..ROUNDS {
                let (again, came) =
                    wake.wait_timeout_while(mutex, guard, Duration::from_secs(5), |sent| {
                        sent.is_none()
                    });
                guard = again;
                assert!(!came, "notified, not due");
                let woke = guard.take().map(|sent| sent.elapsed()).unwrap_or_default();
                answer.send(woke).expect("the notifier waits");
            }
        })
    };
    let mut rounds = Rounds::crossing(1);
    for _ in 0..ROUNDS {
        // The waiter back in its wait before the notify; spun, not slept.
        spin(Duration::from_micros(300));
        let from = ProcessorTime::process();
        *shared.0.locked() = Some(Instant::now());
        shared.1.notify_one();
        let woke = answered
            .recv_timeout(Duration::from_secs(5))
            .expect("woken");
        rounds.took.push(woke);
        rounds.worked.push(from.elapsed());
        rounds.beside();
    }
    waiter.join().expect("woken");
    rounds.hold("a notify to the waiting thread running");
}
