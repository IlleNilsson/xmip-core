# xmip-core

Core Xmip identifiers, shared types and stable public contracts, a file for
each: the identifiers every other crate keys by, each a UUIDv7 read strictly
and written without allocating, and the `IdGenerator` that mints them
(`id.rs`); the `Severity` and `ExecutionPhase` every audit record carries;
the one retryable failure, `Failure`, and the macros that declare a crate's
error in one line (`error.rs`, ADR-0037); what time it is, in the estate's
one unit — nanoseconds since the Unix epoch as an `i128` — from the `Clock`
trait and its one production reading, `SystemClock` (`clock.rs`, behind the
default `std` feature; a wire that counts seconds converts at the wire with
`Clock::unix_seconds`); the identity vocabulary the three gates share, with
the catalog of the mechanisms Xmip implements (`mechanism/declared.rs`,
ADR-0050); and `settings`: the
shape in which every technology of every capability declares what a
Location may set — each setting's name, kind, default or requirement,
meaning and side — and the one reading of a Location's values through it
(ADR-0064, amendment 2026-09-26). And `ScalarValue`, the one value a
promoted property and a structured field both are, with its one rendering
as text, `ScalarValue::text`, which a filter's comparison and a path's write
both use; each caller decides what a `Null` is. It goes first,
because every other repository depends on it (`repository-model.md` section
10).

It implements nothing: no provider, no protocol, no capability. The identity
vocabulary lives here rather than in `xmip-core-party` because identify,
authenticate and authorize need it and none of them may depend on the Party —
authenticating a credential is not the same as knowing whose it is.

ADR-0019 makes the Party the identity holder in both directions, ADR-0022
classifies every identity by how it is proven, and ADR-0009 keeps roles out of
it; `architecture.toml` carries the maturity.
