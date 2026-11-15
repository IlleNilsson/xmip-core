//! How near real time a measurement's rounds were: the figure a test of the
//! millisecond rule holds, the distribution it prints, and the plain wake
//! it is held beside ([`PlainWake`]). Compiled for tests only, with
//! `test-support`; every latency test in the estate reads its rounds
//! through [`spread`], so the figure is computed once.
//!
//! What is held is [`Spread::fastest_of_five`]: the median of the fastest
//! round in each five, computed by `crate::fastest`, which a running node's
//! own report reads too. A poll, a missed notify or a busy loop is in every
//! round, and so in the fastest of each five too; a wake that waited its
//! turn for a processor on a loaded machine is in some rounds only, and is
//! not. The median, the 99th percentile and the worst are printed and held
//! by nothing: on a machine busy on every processor they measure the
//! scheduler (CONTRIBUTING.md: a millisecond *apart from load*).

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::vec::Vec;

use crate::fastest::{FIVE, FastestOfFive};
use crate::processor_time::ProcessorTime;

/// A distribution of measured rounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Spread {
    /// The median of the fastest round in each [`FIVE`], in the order the
    /// rounds were measured: what a test holds.
    pub fastest_of_five: Duration,
    /// The median of every round.
    pub median: Duration,
    /// The 99th percentile of every round.
    pub p99: Duration,
    /// The slowest round.
    pub worst: Duration,
    /// How many rounds were measured.
    pub rounds: usize,
}

/// The spread of `taken`, in the order the rounds were measured, printed
/// under `what`, because a mean hides the tail an operator waits on.
///
/// # Panics
/// Where nothing was measured.
#[must_use]
pub fn spread(what: &str, taken: Vec<Duration>) -> Spread {
    assert!(!taken.is_empty(), "{what}: nothing was measured");
    let mut fastest = FastestOfFive::new();
    for round in &taken {
        fastest.push(*round);
    }
    let mut sorted = taken;
    sorted.sort();
    let at = |percent: usize| sorted[(sorted.len() - 1) * percent / 100];
    let spread = Spread {
        fastest_of_five: fastest.read().expect("measured"),
        median: at(50),
        p99: at(99),
        worst: at(100),
        rounds: sorted.len(),
    };
    std::println!(
        "{what}: the fastest of five {:?}; median {:?}, p99 {:?}, worst {:?} over {}",
        spread.fastest_of_five,
        spread.median,
        spread.p99,
        spread.worst,
        spread.rounds
    );
    spread
}

/// The machine's own wake beside a measured round: a thread blocked on a
/// plain channel, woken by a stamp, which answers on another. Measured
/// round by round beside what a test measures, under the same load.
pub struct PlainWake {
    tell: mpsc::Sender<Instant>,
    back: mpsc::Receiver<Duration>,
}

impl PlainWake {
    /// A waiting thread.
    ///
    /// # Panics
    /// Where a thread cannot be started, which a test cannot do without.
    #[must_use]
    pub fn start() -> Self {
        let (tell, told) = mpsc::channel::<Instant>();
        let (answer, back) = mpsc::channel();
        thread::Builder::new()
            .name("plain wake".into())
            .spawn(move || {
                for sent in told {
                    if answer.send(sent.elapsed()).is_err() {
                        break;
                    }
                }
            })
            .expect("a thread");
        Self { tell, back }
    }

    /// One round: the wake, from the stamp to the waiting thread running,
    /// and this process's processor time across the round, the answer back
    /// included.
    ///
    /// # Panics
    /// Where the waiting thread does not answer within five seconds.
    #[must_use]
    pub fn round(&self) -> (Duration, Duration) {
        let from = ProcessorTime::process();
        self.tell.send(Instant::now()).expect("the waiter waits");
        let woke = self
            .back
            .recv_timeout(Duration::from_secs(5))
            .expect("the waiter woke");
        (woke, from.elapsed())
    }
}

/// The millisecond of the millisecond rule (`CONTRIBUTING.md`).
const MILLISECOND: Duration = Duration::from_millis(1);

/// A wait's rounds and a [`PlainWake`] beside each, held to the
/// millisecond rule by what is Xmip's alone, each figure the fastest round
/// in every five: a poll or a missed notify is in every round, a wake that
/// waited its turn for a processor in some only. The one rule for a wait
/// that works as it wakes, estate-wide; until 2026-11-14 the operate
/// plane's and the cluster's timing programs each held a copy of it, and
/// Xmip Storage's wakes were held to a wall-clock tail instead.
///
/// - **The wake is a wake**: each round's wall clock is under the
///   millisecond plus a plain wake for each thread the wait wakes where the
///   plain wake wakes one ([`Rounds::crossing`]).
/// - **Xmip's own work is under the millisecond**: this process's
///   processor time across a round is under the millisecond plus the
///   process's across the plain wake, which counts what the rest of the
///   process runs beside it.
///
/// The median and the tail are printed and held by nothing: on a machine
/// busy on every processor they measure the scheduler.
pub struct Rounds {
    /// Each round's wall clock.
    pub took: Vec<Duration>,
    /// This process's processor time across each round
    /// ([`ProcessorTime::process`]).
    pub worked: Vec<Duration>,
    wakes: u32,
    plain: PlainWake,
    woke: Vec<Duration>,
    idle: Vec<Duration>,
}

impl Rounds {
    /// The rounds of a wait that wakes `wakes` threads where a plain wake
    /// wakes one.
    #[must_use]
    pub fn crossing(wakes: u32) -> Self {
        Self {
            took: Vec::new(),
            worked: Vec::new(),
            wakes,
            plain: PlainWake::start(),
            woke: Vec::new(),
            idle: Vec::new(),
        }
    }

    /// One plain wake, after a round.
    pub fn beside(&mut self) {
        let (woke, idle) = self.plain.round();
        self.woke.push(woke);
        self.idle.push(idle);
    }

    /// One round, a call that waits on what it asks: its wall clock and
    /// this process's processor time across it, then a plain wake beside
    /// it; what the call answered. A round whose start and end two threads
    /// stamp pushes [`Rounds::took`] and [`Rounds::worked`] itself and
    /// calls [`Rounds::beside`].
    pub fn round<T>(&mut self, call: impl FnOnce() -> T) -> T {
        let from = ProcessorTime::process();
        let began = Instant::now();
        let answer = call();
        self.took.push(began.elapsed());
        self.worked.push(from.elapsed());
        self.beside();
        answer
    }

    /// Held, printed under `what`: its wall clock's spread.
    ///
    /// # Panics
    /// Where a round's wall clock or its processor time is a millisecond or
    /// more beyond the plain wake's, the fastest of five, or nothing was
    /// measured.
    pub fn hold(self, what: &str) -> Spread {
        let took = spread(what, self.took);
        let usual = spread("a plain wake beside it", self.woke).fastest_of_five;
        let worked = spread("the process's processor time for a round", self.worked);
        woke_beside_a_plain_wake(what, took.fastest_of_five, usual, self.wakes);
        worked_beside_a_plain_wake(
            &std::format!("{what}: the process's processor time"),
            worked.fastest_of_five,
            self.idle,
        );
        took
    }
}

/// A wake held to the millisecond rule beside the machine's: `took`, the
/// fastest of five of a round's wall clock, under the millisecond plus
/// `usual`, the fastest of five of a plain wake measured in the same rounds,
/// once for each of the `wakes` threads the round wakes. The half of
/// [`Rounds::hold`] that a round whose processor time this process cannot
/// count apart is held by.
///
/// # Panics
/// Where `took` is a millisecond or more beyond the plain wakes'.
pub fn woke_beside_a_plain_wake(what: &str, took: Duration, usual: Duration, wakes: u32) {
    assert!(
        took < MILLISECOND + usual * wakes,
        "{what}: the fastest of five {took:?}, the machine's own {usual:?}, {wakes} woken"
    );
}

/// Xmip's own work held to the millisecond rule beside the machine's:
/// `worked`, the fastest of five of the processor time a round took, under
/// the millisecond plus `idle`'s, this process's processor time across a
/// plain wake measured in the same rounds ([`PlainWake::round`]), the
/// fastest of five. The half of [`Rounds::hold`] that a figure measured
/// elsewhere is held by: a node's own share of a Message, as its stop
/// record says it.
///
/// # Panics
/// Where `worked` is a millisecond or more beyond the plain wake's, or no
/// plain wake was measured.
pub fn worked_beside_a_plain_wake(what: &str, worked: Duration, idle: Vec<Duration>) {
    let idle = spread("the process's processor time for a plain wake", idle).fastest_of_five;
    assert!(
        worked < MILLISECOND + idle,
        "{what} {worked:?}, for a plain wake {idle:?}"
    );
}

/// What an answer said at once — from what is held, with no wait — took:
/// `call` made [`FIVE`] times, the fastest wall clock of the five and the
/// most processor time the calling thread spent in one, printed under
/// `what`, with the last answer. A wait (a connection tried, a timeout
/// waited out) is in every call and so in the fastest; a preemption on a
/// busy machine is in some only, and costs the thread no processor time.
///
/// # Panics
/// As [`ProcessorTime::thread`].
pub fn at_once<T>(what: &str, mut call: impl FnMut() -> T) -> (T, Duration, Duration) {
    let mut fastest = Duration::MAX;
    let mut most = Duration::ZERO;
    let mut answer = None;
    for _ in 0..FIVE {
        let started = Instant::now();
        let spent = ProcessorTime::thread();
        answer = Some(call());
        most = most.max(spent.elapsed());
        fastest = fastest.min(started.elapsed());
    }
    std::println!(
        "{what}: the fastest of five {fastest:?}, the most processor time in one {most:?}"
    );
    (answer.expect("called five times"), fastest, most)
}

/// Spin, rather than sleep, for `pause`: a sleep on a busy machine is
/// milliseconds, which would space the rounds by the scheduler rather than
/// by the test.
pub fn spin(pause: Duration) {
    let started = Instant::now();
    while started.elapsed() < pause {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spread_is_the_fastest_of_five_the_median_the_tail_and_the_worst() {
        let taken = (1..=100).map(Duration::from_micros).collect();
        let spread = spread("one to a hundred", taken);
        assert_eq!(spread.median, Duration::from_micros(50));
        assert_eq!(spread.p99, Duration::from_micros(99));
        assert_eq!(spread.worst, Duration::from_micros(100));
        assert_eq!(spread.rounds, 100);
        // The fastest of 1..=5, 6..=10, ...: 1, 6, ..., 96; the middle of
        // the twenty is the tenth, 46.
        assert_eq!(spread.fastest_of_five, Duration::from_micros(46));
    }

    #[test]
    fn a_slow_round_in_every_five_moves_the_median_and_not_the_fastest() {
        let taken = (0..100)
            .map(|round| Duration::from_micros(if round % 5 < 3 { 900 } else { 100 }))
            .collect();
        let spread = spread("three slow in every five", taken);
        assert_eq!(spread.median, Duration::from_micros(900));
        assert_eq!(spread.fastest_of_five, Duration::from_micros(100));
    }

    #[test]
    fn an_answer_at_once_is_called_five_times_and_a_wait_is_in_the_fastest() {
        let mut calls = 0;
        let (last, fastest, _) = at_once("a sleep of 3 ms", || {
            calls += 1;
            std::thread::sleep(Duration::from_millis(3));
            calls
        });
        assert_eq!((calls, last), (FIVE, FIVE));
        assert!(fastest >= Duration::from_millis(3), "{fastest:?}");
    }

    /// Rounds of `took` and `worked` each, a plain wake measured beside
    /// each.
    fn rounds(took: Duration, worked: Duration) -> Rounds {
        let mut rounds = Rounds::crossing(1);
        for _ in 0..20 {
            rounds.took.push(took);
            rounds.worked.push(worked);
            rounds.beside();
        }
        rounds
    }

    #[test]
    fn rounds_are_held_by_their_wake_and_their_processor_time_beside_a_plain_wake() {
        let quick = Duration::from_micros(100);
        let held = rounds(quick, quick).hold("a quick wait");
        assert_eq!((held.fastest_of_five, held.rounds), (quick, 20));
        let late = Duration::from_secs(2);
        let slept = std::panic::catch_unwind(|| rounds(late, quick).hold("a wait woken late"));
        assert!(slept.is_err(), "a wake seconds beyond a plain wake fails");
        let busy = std::panic::catch_unwind(|| rounds(quick, late).hold("a wait that worked"));
        assert!(busy.is_err(), "a round's work beyond a plain wake's fails");
    }

    #[test]
    fn a_round_is_its_call_measured_with_a_plain_wake_beside_it() {
        let mut rounds = Rounds::crossing(1);
        let answers: Vec<usize> = (0..10).map(|round| rounds.round(|| round * 2)).collect();
        assert_eq!(answers, (0..10).map(|round| round * 2).collect::<Vec<_>>());
        assert_eq!((rounds.took.len(), rounds.worked.len()), (10, 10));
        assert_eq!((rounds.woke.len(), rounds.idle.len()), (10, 10));
        assert_eq!(rounds.hold("a call answered at once").rounds, 10);
    }

    #[test]
    fn a_wake_is_held_beside_a_plain_wake_for_each_thread_it_wakes() {
        let usual = Duration::from_micros(50);
        woke_beside_a_plain_wake("two woken", Duration::from_micros(1090), usual, 2);
        let beyond = std::panic::catch_unwind(|| {
            woke_beside_a_plain_wake("one woken", Duration::from_micros(1090), usual, 1);
        });
        assert!(beyond.is_err(), "beyond the plain wakes' fails");
    }

    #[test]
    fn work_measured_elsewhere_is_held_beside_the_plain_wake_measured_here() {
        let idle: Vec<Duration> = (0..10).map(|_| Duration::from_micros(50)).collect();
        worked_beside_a_plain_wake("a share", Duration::from_micros(1040), idle.clone());
        let beyond = std::panic::catch_unwind(|| {
            worked_beside_a_plain_wake("a share", Duration::from_micros(1060), idle);
        });
        assert!(beyond.is_err(), "beyond the plain wake's fails");
    }

    #[test]
    fn a_plain_wake_is_measured_every_round() {
        let plain = PlainWake::start();
        let rounds: Vec<(Duration, Duration)> = (0..20).map(|_| plain.round()).collect();
        assert_eq!(rounds.len(), 20);
        assert!(
            rounds
                .iter()
                .all(|(woke, _)| *woke < Duration::from_secs(5))
        );
    }
}
