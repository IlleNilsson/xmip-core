//! Where an execution stands.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionPhase {
    Begin,
    Execute,
    Finished,
    Failure,
}
