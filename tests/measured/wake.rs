//! A thread waiting under a lock until woken or due (`xmip_core::wake`),
//! held to the millisecond.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use xmip_core::lock::Locked;
use xmip_core::wake::Wake;

/// The median, the 99th percentile and the worst of `taken`.
fn spread(mut taken: Vec<Duration>) -> (Duration, Duration, Duration) {
    taken.sort();
    let at = |percent: usize| taken[(taken.len() - 1) * percent / 100];
    (at(50), at(99), at(100))
}

/// A due time is kept within a millisecond, apart from load: held to
/// `std::thread::sleep` measured beside it, which on Windows already waits
/// on a high-resolution timer. Until 2026-10-16 a timed wait ended on the
/// system tick, up to 9-14 ms late.
#[test]
fn a_wait_ends_within_a_millisecond_of_its_due_time() {
    const DUE: Duration = Duration::from_millis(2);
    let wake = Wake::new();
    let mutex = Mutex::new(());
    let (mut late, mut control) = (Vec::new(), Vec::new());
    for _ in 0..200 {
        let began = Instant::now();
        let (guard, came) = wake.wait_timeout(&mutex, mutex.locked(), DUE);
        late.push(began.elapsed().saturating_sub(DUE));
        assert!(came, "nothing notified");
        drop(guard);
        let began = Instant::now();
        thread::sleep(DUE);
        control.push(began.elapsed().saturating_sub(DUE));
    }
    let ((median, tail, _), (usual, _, worst)) = (spread(late), spread(control));
    println!(
        "a 2 ms wait late: median {median:?}, p99 {tail:?}; \
         thread::sleep beside it: median {usual:?}, worst {worst:?}"
    );
    // The median to the millisecond; the tail, which load alone moves by
    // milliseconds either way, within the worst the machine did.
    assert!(
        median < Duration::from_millis(1) + usual,
        "median {median:?} late, the machine's own {usual:?}"
    );
    assert!(
        tail < Duration::from_millis(1) + worst,
        "p99 {tail:?} late, the machine's worst {worst:?}"
    );
}

/// A notify wakes a waiting thread within a millisecond, apart from load,
/// and says it was not the due time.
#[test]
fn a_notify_wakes_within_a_millisecond() {
    let shared = Arc::new((Mutex::new(None::<Instant>), Wake::new()));
    let waiter = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            let (mutex, wake) = &*shared;
            let mut taken = Vec::new();
            let mut guard = mutex.locked();
            for _ in 0..200 {
                let (again, came) =
                    wake.wait_timeout_while(mutex, guard, Duration::from_secs(5), |sent| {
                        sent.is_none()
                    });
                guard = again;
                assert!(!came, "notified, not due");
                taken.push(guard.take().map(|sent| sent.elapsed()).unwrap_or_default());
            }
            taken
        })
    };
    for _ in 0..200 {
        while shared.0.locked().is_some() {
            thread::sleep(Duration::from_micros(100));
        }
        thread::sleep(Duration::from_micros(300));
        *shared.0.locked() = Some(Instant::now());
        shared.1.notify_one();
    }
    let taken = spread(waiter.join().expect("woken")).1;
    assert!(taken < Duration::from_millis(5), "p99 {taken:?}");
}
