//! Xmip's own share of one Message: the processor time the node's threads
//! spent working for it, apart from the I/O its payload needs — the figure
//! the millisecond rule holds (`CONTRIBUTING.md`, *Rust is here for one
//! reason*: a millisecond *apart from the load it carries*; the owner,
//! 2026-11-01: *the processor time its threads spent working for the
//! Message*).
//!
//! A Message's account is opened when its transport hands it over
//! ([`Spent::begin`]). Each thread that works for it enters the account
//! ([`Spent::entered`]), and what that thread runs meanwhile — counted by
//! the operating system only while it runs (`crate::processor_time`) — is
//! the Message's, except inside each I/O it makes ([`on_io`]): its Stream
//! read from its transport and closed there, its far end told how its
//! receive cycle ended, the Ledger's appends and syncs, its send. A
//! thread waiting — for a wake, a lock, a disk — runs nothing, so what other
//! programs do on the machine, and how long a woken thread waits for a
//! processor, never count. What another thread does for many Messages at
//! once — a Storage node's batch — is counted on an account of its own and
//! shared among them ([`Spent::add`]).
//!
//! Not counted: the scheduler's and an interrupt's time the operating
//! system charges to a thread it wakes, which is a few microseconds at a
//! time; and whatever a transport does before it hands the Message over.
//!
//! A thread that entered no account pays nothing: [`on_io`] only runs what
//! it is given.
//!
//! **A wait for a storage device to sync is the disk's time, never Xmip's**
//! (the owner, 2026-11-09; ADR-0018, amendment 2026-11-09): every sync Xmip
//! makes — a file's, a directory's, the Ledger's — runs in [`on_sync`],
//! which runs it as an I/O and adds the time it waited, by the clock, to
//! the account's disk time ([`Spent::disk_sync`]), reported beside its own
//! share and never held to the millisecond. Nothing else times a sync.

use core::cell::{Cell, RefCell};
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;
use std::sync::Arc;
use std::time::Instant;

use crate::processor_time::ProcessorTime;

/// One Message's account: when it began, the processor time worked for it
/// so far, and the time it waited for a storage device to sync, each in
/// nanoseconds.
#[derive(Clone, Debug)]
pub struct Spent(Arc<Account>);

#[derive(Debug)]
struct Account {
    begun: Instant,
    worked: AtomicU64,
    synced: AtomicU64,
}

std::thread_local! {
    /// The account this thread works for now.
    static ENTERED: RefCell<Option<Spent>> = const { RefCell::new(None) };
    /// Since when this thread's processor time is that account's: `None`
    /// while an I/O runs, or where it works for none.
    static MARK: Cell<Option<ProcessorTime>> = const { Cell::new(None) };
}

impl Spent {
    /// A Message's account, opened now.
    #[must_use]
    pub fn begin() -> Self {
        Self(Arc::new(Account {
            begun: Instant::now(),
            worked: AtomicU64::new(0),
            synced: AtomicU64::new(0),
        }))
    }

    /// When it was opened.
    #[must_use]
    pub fn begun(&self) -> Instant {
        self.0.begun
    }

    /// `worked`, processor time worked for it on a thread that worked for
    /// many at once, its share of it.
    pub fn add(&self, worked: Duration) {
        self.0.worked.fetch_add(nanos(worked), Ordering::Relaxed);
    }

    /// This thread working for it until the guard is dropped: what it runs
    /// meanwhile, outside its I/O, is the account's.
    pub fn entered(&self) -> Entered {
        credit();
        let before = ENTERED.with(|entered| entered.borrow_mut().replace(self.clone()));
        MARK.set(ProcessorTime::try_thread());
        Entered { before }
    }

    /// Xmip's own share of it: the processor time worked for it so far,
    /// and `beside`, its share of what was worked for it elsewhere.
    #[must_use]
    pub fn own(&self, beside: Duration) -> Duration {
        Duration::from_nanos(self.0.worked.load(Ordering::Relaxed)) + beside
    }

    /// The disk's time of it: the time its threads waited for a storage
    /// device to sync so far ([`on_sync`]), and `beside`, its share of a
    /// sync made elsewhere for many at once.
    #[must_use]
    pub fn disk_sync(&self, beside: Duration) -> Duration {
        Duration::from_nanos(self.0.synced.load(Ordering::Relaxed)) + beside
    }
}

/// A duration in nanoseconds, the most a `u64` holds at most.
fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

/// A thread working for a Message's account ([`Spent::entered`]); what it
/// worked for before, once dropped.
#[must_use]
pub struct Entered {
    before: Option<Spent>,
}

impl Drop for Entered {
    fn drop(&mut self) {
        credit();
        let before = self.before.take();
        let working = before.is_some();
        ENTERED.with(|entered| *entered.borrow_mut() = before);
        MARK.set(working.then(ProcessorTime::try_thread).flatten());
    }
}

/// What this thread ran since its mark, credited to the account it works
/// for, and its mark moved to now.
fn credit() {
    let Some(mark) = MARK.get() else {
        return;
    };
    let Some(now) = ProcessorTime::try_thread() else {
        return;
    };
    ENTERED.with(|entered| {
        if let Some(spent) = entered.borrow().as_ref() {
            spent.add(now.since(mark));
        }
    });
    MARK.set(Some(now));
}

/// The account this thread works for now, if any.
#[must_use]
pub fn current() -> Option<Spent> {
    ENTERED.with(|entered| entered.borrow().clone())
}

/// `io` run, its processor time not the account's this thread works for:
/// the payload's own I/O. Within another such I/O it is part of it.
pub fn on_io<T>(io: impl FnOnce() -> T) -> T {
    if MARK.get().is_none() {
        return io();
    }
    credit();
    MARK.set(None);
    let done = io();
    MARK.set(ProcessorTime::try_thread());
    done
}

/// `sync` run, a wait for a storage device to sync — `fsync`,
/// `FlushFileBuffers`, a directory's sync, the Ledger's — as an I/O
/// ([`on_io`]), and the time it waited, by the clock, added to the disk's
/// time of the account this thread works for ([`Spent::disk_sync`]). Every
/// sync Xmip makes runs here; where the thread works for no account, it is
/// only run.
pub fn on_sync<T>(sync: impl FnOnce() -> T) -> T {
    let Some(spent) = current() else {
        return on_io(sync);
    };
    let began = Instant::now();
    let done = on_io(sync);
    spent
        .0
        .synced
        .fetch_add(nanos(began.elapsed()), Ordering::Relaxed);
    done
}

/// A reader whose every read is an I/O ([`on_io`]), and its closing: a
/// Stream read from its transport, and let go of there — a file closed,
/// which the operating system's file system does.
pub struct ReadOnIo<R>(Option<R>);

impl<R> ReadOnIo<R> {
    /// `reader`, each read of it and its closing an I/O.
    pub const fn new(reader: R) -> Self {
        Self(Some(reader))
    }
}

impl<R: std::io::Read> std::io::Read for ReadOnIo<R> {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        match self.0.as_mut() {
            Some(reader) => on_io(|| reader.read(into)),
            None => Ok(0),
        }
    }
}

impl<R> Drop for ReadOnIo<R> {
    fn drop(&mut self) {
        let reader = self.0.take();
        on_io(|| drop(reader));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `for_` of this thread's processor time: by its own count and not
    /// the clock's, so a thread another test preempts still runs all of it.
    fn spin(for_: Duration) {
        let began = ProcessorTime::thread();
        while ProcessorTime::thread().since(began) < for_ {
            core::hint::spin_loop();
        }
    }

    #[test]
    fn work_for_many_is_counted_on_an_account_of_its_own_and_shared() {
        let batch = Spent::begin();
        {
            let _entered = batch.entered();
            spin(Duration::from_millis(5));
        }
        let worked = batch.own(Duration::ZERO);
        assert!(worked > Duration::from_millis(3), "{worked:?}");
        let spent = Spent::begin();
        spent.add(worked / 2);
        assert_eq!(spent.own(worked / 2), worked / 2 * 2);
    }

    #[test]
    fn a_sync_waited_is_the_disks_time_of_the_account_worked_for() {
        let spent = Spent::begin();
        // Outside any account: run, and counted nowhere.
        assert_eq!(on_sync(|| 7), 7);
        {
            let _entered = spent.entered();
            on_sync(|| std::thread::sleep(Duration::from_millis(2)));
        }
        let disk = spent.disk_sync(Duration::ZERO);
        assert!(disk >= Duration::from_millis(2), "{disk:?}");
        assert_eq!(spent.disk_sync(disk), disk * 2, "and its share beside");
    }
}
