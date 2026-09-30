use crate::resource::ResourceId;

/// Canonical identity of an operation slot and its generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationId {
    slot: usize,
    generation: u64,
}

impl OperationId {
    /// Creates an identity for an operation slot.
    #[must_use]
    pub const fn new(slot: usize, generation: u64) -> Self {
        Self { slot, generation }
    }

    pub(crate) const fn slot(self) -> usize {
        self.slot
    }

    pub(crate) const fn generation(self) -> u64 {
        self.generation
    }
}

/// Lifecycle state of a registered operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    /// The slot is available for registration.
    Vacant,
    /// The declaration is registered but has not been admitted.
    Ready,
    /// The operation holds its complete declared reservation.
    Running,
    /// The operation published successfully.
    Committed,
    /// The operation was discarded or aborted.
    Failed,
}

/// Caller-owned declaration and lifecycle storage for one operation slot.
///
/// Declarations are borrowed for the model's lifetime. Callers must not
/// mutate the declaration slices through an alias while the model lives.
pub struct Operation<'a> {
    pub(crate) reads: Option<&'a [ResourceId]>,
    pub(crate) writes: Option<&'a [ResourceId]>,
    pub(crate) generation: u64,
    pub(crate) state: State,
}

impl Operation<'_> {
    /// Creates an unused operation slot at generation zero.
    #[must_use]
    pub const fn vacant() -> Self {
        Self::vacant_with_generation(0)
    }

    /// Creates an unused operation slot with a caller-selected generation.
    #[must_use]
    pub const fn vacant_with_generation(generation: u64) -> Self {
        Self {
            reads: None,
            writes: None,
            generation,
            state: State::Vacant,
        }
    }
}
