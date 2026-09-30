/// Canonical identity of a resource slot and its caller-supplied generation.
///
/// An identifier is only an identity token, not an authority to access a
/// resource. A model validates both components before using it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceId {
    slot: usize,
    generation: u64,
}

impl ResourceId {
    /// Creates an identity for a resource slot.
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

/// Caller-owned committed storage for one whole-object resource.
///
/// The model borrows a resource slice exclusively. Callers must not create
/// two resource slots that refer to overlapping external storage, and must
/// not mutate the slice or its values through an alias while the model lives.
#[derive(Clone, Copy, Debug)]
pub struct Resource<T: Copy> {
    generation: u64,
    value: T,
}

impl<T: Copy> Resource<T> {
    /// Creates a resource with its initial committed value.
    #[must_use]
    pub const fn new(generation: u64, value: T) -> Self {
        Self { generation, value }
    }

    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) const fn value(&self) -> T {
        self.value
    }

    pub(crate) fn set_value(&mut self, value: T) {
        self.value = value;
    }
}
