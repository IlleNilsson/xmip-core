//! How near real time a measurement's rounds were: the figure a test of the
//! millisecond rule holds, the distribution it prints, and the plain wake
//! it is held beside ([`PlainWake`]). Compiled for tests only, with
//! `test-support`; every latency test in the estate reads its rounds
//! through [`spread`], so the figure is computed once.
//!
//! What is held is [`Spread::fastest_of_five`]: the median of the fastest
//! round in each five. A poll, a missed notify or a busy loop is in every
//! round, and so in the fastest of each five too; a wake that waited its
//! turn for a processor on a loaded machine is in some rounds only, and is
//! not. The median, the 99th percentile and the worst are printed and held
//! by nothing: on a machine busy on every processor they measure the
//! scheduler (CONTRIBUTING.md: a millisecond *apart from load*).

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::vec::Vec;

use crate::processor_time::ProcessorTime;

/// How many rounds the fastest is taken from.
pub const FIVE: usize = 5;

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
    let mut fastest: Vec<Duration> = taken
        .chunks(FIVE)
        .filter_map(|five| five.iter().min().copied())
        .collect();
    fastest.sort();
    let mut sorted = taken;
    sorted.sort();
    let at = |percent: usize| sorted[(sorted.len() - 1) * percent / 100];
    let spread = Spread {
        fastest_of_five: fastest[(fastest.len() - 1) / 2],
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
