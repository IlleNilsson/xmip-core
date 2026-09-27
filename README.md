# xmip-core

Core Xmip identifiers, shared types and stable public contracts: the
identifiers every other crate keys by, the `Severity` and `ExecutionPhase`
every audit record carries, `ExecutionScope`, the `Clock` and `IdGenerator`
traits, the identity vocabulary the three gates share, and `settings`: the
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
