# xmip-core

Core Xmip identifiers, shared types and stable public contracts: the
identifiers every other crate keys by, the `Severity` and `ExecutionPhase`
every audit record carries, `ExecutionScope`, the `Clock` and `IdGenerator`
traits, and the identity vocabulary the three gates share. It goes first,
because every other repository depends on it (`repository-model.md` section
10).

It implements nothing: no provider, no protocol, no capability. The identity
vocabulary lives here rather than in `xmip-core-party` because identify,
authenticate and authorize need it and none of them may depend on the Party —
authenticating a credential is not the same as knowing whose it is.

ADR-0019 makes the Party the identity holder in both directions, ADR-0022
classifies every identity by how it is proven, and ADR-0009 keeps roles out of
it; `architecture.toml` carries the maturity.
