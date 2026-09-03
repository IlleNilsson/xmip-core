//! The two directions: how a Stream got in, and how a Message goes out.
//!
//! ADR-0019 is *Identity, Parties and the two directions*, and `terminology.md`
//! files arrivals and departures together for the same reason — an operator
//! watching an estate is watching things come in and things go out.
//!
//! **The two are deliberately not the same three.** A Stream can arrive by
//! being detected; a Message cannot depart that way, because nothing outside
//! Xmip is watching on Xmip's behalf. Collection replaces detection, and the
//! difference is operational: a pushed departure fails at Xmip and is Xmip's to
//! retry, a collected one waits, and its failure mode is nobody turning up.
//!
//! Separate from how an identity was established — see [`crate::established`].

use core::fmt;

/// How a Stream got into Xmip.
///
/// Three ways in, and only one of them is something turning up. A scheduled
/// pickup has no caller at all — Xmip is the client, and there is nobody on the
/// other end to present anything. Recording which of the three happened is what
/// makes the identity that follows explicable: an identity with nobody to have
/// passed it had better be inferred.
///
/// All three are arrivals. The Stream arrives at Xmip; this says how it got
/// there.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Arriving {
    /// Something connected and sent it. HTTP, SOAP, gRPC, AS2, MLLP, a queue
    /// consumer with a live producer on the other side.
    Pushed,
    /// Xmip was watching and it appeared. A folder, a queue, a table, an inbox.
    /// Nobody connected; the Stream was simply there when Xmip looked.
    Detected,
    /// A timer fired and Xmip went and fetched it. Xmip is the client, so the
    /// credential in play is Xmip's own and proves nothing about the source.
    Scheduled,
}

impl fmt::Display for Arriving {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pushed => "pushed",
            Self::Detected => "detected",
            Self::Scheduled => "scheduled",
        })
    }
}

/// How a Stream leaves Xmip.
///
/// The mirror of [`Arriving`], and deliberately not the same three words. A
/// Stream can arrive by being detected, because Xmip can watch; it cannot
/// depart by being detected, because nothing outside Xmip is watching on Xmip's
/// behalf. What replaces it is collection — Xmip holds the Stream and something
/// comes and takes it.
///
/// ```text
/// Pushed      Xmip connects and sends it
/// Collected   Xmip holds it and something comes and gets it
/// Scheduled   a timer fires and Xmip sends what has accumulated
/// ```
///
/// The distinction is not cosmetic. A pushed departure fails at Xmip and is
/// Xmip's to retry; a collected one waits, and its failure mode is nobody
/// turning up. Reporting them as one number makes an unreachable partner and an
/// idle one look identical on the same dashboard.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Departing {
    /// Xmip connects and sends. HTTP, SFTP, AS2, a queue producer.
    Pushed,
    /// Xmip holds it and something comes and gets it — a solicit-response
    /// reply, a partner polling an outbox, a client calling an API.
    ///
    /// Xmip is the server, so it presents no identity: the collector does, and
    /// is put through the same three gates an arrival is.
    Collected,
    /// A timer fires and Xmip sends what has accumulated.
    Scheduled,
}

impl fmt::Display for Departing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pushed => "pushed",
            Self::Collected => "collected",
            Self::Scheduled => "scheduled",
        })
    }
}

