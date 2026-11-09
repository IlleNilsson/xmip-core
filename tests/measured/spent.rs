//! What one Message spends on its own I/O, Xmip's own share beside it, held
//! to the processor time a thread runs, and the disk's time apart from it
//! (`xmip_core::spent`).

use std::time::Duration;

use xmip_core::processor_time::ProcessorTime;
use xmip_core::spent::{ReadOnIo, Spent, current, on_io, on_sync};

/// Run `for_` of this thread's processor time: by its own count and not
/// the clock's, so a thread preempted still runs all of it.
fn spin(for_: Duration) {
    let began = ProcessorTime::thread();
    while ProcessorTime::thread().since(began) < for_ {
        core::hint::spin_loop();
    }
}

#[test]
fn what_a_thread_runs_for_it_counts_and_its_io_and_its_waits_do_not() {
    let spent = Spent::begin();
    {
        let _entered = spent.entered();
        spin(Duration::from_millis(5));
        on_io(|| spin(Duration::from_millis(20)));
        std::thread::sleep(Duration::from_millis(20));
    }
    spin(Duration::from_millis(20));
    let own = spent.own(Duration::ZERO);
    assert!(own > Duration::from_millis(3), "the spin it ran: {own:?}");
    assert!(own < Duration::from_millis(15), "no I/O, no wait: {own:?}");
    assert!(
        current().is_none(),
        "the thread works for nothing once left"
    );
}

/// A reader whose reads and whose closing each take `for_` of the thread's
/// processor time.
struct Slow(Duration);

impl std::io::Read for Slow {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        spin(self.0);
        Ok(0)
    }
}

impl Drop for Slow {
    fn drop(&mut self) {
        spin(self.0);
    }
}

#[test]
fn a_stream_s_reads_and_its_closing_are_its_io() {
    let spent = Spent::begin();
    {
        let _entered = spent.entered();
        let mut read = ReadOnIo::new(Slow(Duration::from_millis(20)));
        let _ = std::io::Read::read(&mut read, &mut [0; 8]);
        drop(read);
    }
    let own = spent.own(Duration::ZERO);
    assert!(own < Duration::from_millis(10), "neither counted: {own:?}");
}

#[test]
fn a_sync_is_the_disks_time_and_never_its_own_share() {
    let spent = Spent::begin();
    {
        let _entered = spent.entered();
        on_sync(|| spin(Duration::from_millis(20)));
    }
    let own = spent.own(Duration::ZERO);
    let disk = spent.disk_sync(Duration::ZERO);
    assert!(
        own < Duration::from_millis(10),
        "not its own share: {own:?}"
    );
    assert!(disk >= Duration::from_millis(20), "the disk's: {disk:?}");
}
