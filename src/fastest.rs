//! The fastest of five: the one figure the millisecond rule holds a
//! measured time to, computed here once, for a running node's own report
//! and for every test that measures (`latency::spread`).
//!
//! Of each [`FIVE`] times in the order they were measured the fastest is
//! kept, and the figure is the median of those kept. A poll, a missed
//! notify or a busy loop is in every time, and so in the fastest of each
//! five too; a wake that waited its turn for a processor on a loaded
//! machine is in some only, and is not (`CONTRIBUTING.md`: a millisecond
//! *apart from load*).

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::time::Duration;

/// How many times the fastest is taken from.
pub const FIVE: usize = 5;

/// Times measured one after another, read as the median of the fastest of
/// each [`FIVE`]: the fives it has finished, and the one under way.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FastestOfFive {
    /// The fastest of each five finished, oldest first.
    fastest: VecDeque<Duration>,
    /// The fastest of the five under way, and how many of it are in.
    current: Option<(Duration, usize)>,
    /// The most fives it keeps, the oldest let go first; `None` for all.
    keeping: Option<usize>,
}

impl FastestOfFive {
    /// One that keeps every five: what a test that measures a known number
    /// of times reads.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fastest: VecDeque::new(),
            current: None,
            keeping: None,
        }
    }

    /// One that keeps the last `fives` fives, so what a node that runs for
    /// months holds stays bounded.
    #[must_use]
    pub const fn keeping(fives: usize) -> Self {
        Self {
            fastest: VecDeque::new(),
            current: None,
            keeping: Some(fives),
        }
    }

    /// `taken`, the next time measured.
    pub fn push(&mut self, taken: Duration) {
        let (fastest, count) = self.current.map_or((taken, 1), |(fastest, count)| {
            (fastest.min(taken), count + 1)
        });
        if count < FIVE {
            self.current = Some((fastest, count));
            return;
        }
        self.current = None;
        self.fastest.push_back(fastest);
        if let Some(most) = self.keeping
            && self.fastest.len() > most.max(1)
        {
            self.fastest.pop_front();
        }
    }

    /// The median of the fastest of each five, the five under way among
    /// them; `None` where nothing was measured.
    #[must_use]
    pub fn read(&self) -> Option<Duration> {
        let mut fastest: Vec<Duration> = self.fastest.iter().copied().collect();
        fastest.extend(self.current.map(|(fastest, _)| fastest));
        fastest.sort();
        fastest.get(fastest.len().checked_sub(1)? / 2).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(times: impl IntoIterator<Item = u64>, mut into: FastestOfFive) -> FastestOfFive {
        for micros in times {
            into.push(Duration::from_micros(micros));
        }
        into
    }

    #[test]
    fn the_figure_is_the_median_of_the_fastest_of_each_five() {
        // The fastest of 1..=5, 6..=10, ...: 1, 6, ..., 96; the middle of
        // the twenty is the tenth, 46.
        let read = of(1..=100, FastestOfFive::new()).read();
        assert_eq!(read, Some(Duration::from_micros(46)));
    }

    #[test]
    fn a_slow_time_in_every_five_moves_nothing() {
        let times = (0..100).map(|n| if n % 5 < 3 { 900 } else { 100 });
        let read = of(times, FastestOfFive::new()).read();
        assert_eq!(read, Some(Duration::from_micros(100)));
    }

    #[test]
    fn the_five_under_way_counts_and_nothing_measured_reads_nothing() {
        assert_eq!(FastestOfFive::new().read(), None);
        let read = of([300, 200], FastestOfFive::new()).read();
        assert_eq!(read, Some(Duration::from_micros(200)));
    }

    #[test]
    fn one_that_keeps_some_fives_lets_the_oldest_go() {
        // Ten fives of 900 and then ten of 100, two kept: the last two.
        let times = (0..100).map(|n| if n < 50 { 900 } else { 100 });
        let read = of(times, FastestOfFive::keeping(2)).read();
        assert_eq!(read, Some(Duration::from_micros(100)));
    }
}
