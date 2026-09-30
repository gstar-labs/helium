use core::marker::PhantomData;

use crate::operation::{Operation, OperationId, State};
use crate::resource::{Resource, ResourceId};

/// Errors returned by checked model operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// A resource identity names no slot in the model.
    ResourceOutOfRange,
    /// A resource identity names an old generation.
    StaleResource,
    /// An operation identity names no slot in the model.
    OperationOutOfRange,
    /// An operation identity names an old generation.
    StaleOperation,
    /// An identifier occurs more than once in one declaration set.
    DuplicateDeclaration,
    /// The requested resource is not declared for reading.
    UndeclaredRead,
    /// The requested resource is not declared for writing.
    UndeclaredWrite,
    /// The operation is not in the lifecycle state required by a transition.
    WrongState,
    /// A complete reservation cannot be admitted alongside a running one.
    Conflict,
    /// Caller-provided storage has no available slot.
    Capacity,
    /// Advancing a generation would overflow.
    GenerationOverflow,
}

/// A caller-owned private staging slot.
///
/// Staged values are visible only through checked model views. The fields are
/// private so callers cannot publish a value without the model transition.
#[derive(Clone, Copy)]
pub struct Stage<T: Copy> {
    marker: PhantomData<T>,
}

impl<T: Copy> Stage<T> {
    /// Creates an empty caller-owned staging slot.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

/// Deterministic reservation model over caller-provided storage.
///
/// The model exclusively borrows its resource, operation, and staging slices
/// for its lifetime. Safe callers therefore cannot mutate those buffers
/// behind the model. Callers remain responsible for avoiding unmediated
/// external aliases outside Rust's checked borrowing rules, and for not
/// registering overlapping resource slots as distinct whole-object resources.
pub struct Model<'a, T: Copy> {
    resources: &'a mut [Resource<T>],
    operations: &'a mut [Operation<'a>],
    _stages: &'a mut [Stage<T>],
}

impl<'a, T: Copy> Model<'a, T> {
    /// Creates a model over caller-owned resource, operation, and staging
    /// storage.
    pub fn new(
        resources: &'a mut [Resource<T>],
        operations: &'a mut [Operation<'a>],
        stages: &'a mut [Stage<T>],
    ) -> Self {
        Self {
            resources,
            operations,
            _stages: stages,
        }
    }

    /// Registers a complete read/write footprint in the first vacant slot.
    ///
    /// Every declaration is checked before the operation slot changes state.
    /// A resource may occur once in each set, but not more than once within a
    /// set.
    ///
    /// # Errors
    ///
    /// Returns an identity, duplicate, or capacity error without registering
    /// a partial declaration.
    pub fn register(
        &mut self,
        reads: &'a [ResourceId],
        writes: &'a [ResourceId],
    ) -> Result<OperationId, Error> {
        self.validate_declaration(reads)?;
        self.validate_declaration(writes)?;

        let Some((slot, operation)) = self
            .operations
            .iter_mut()
            .enumerate()
            .find(|(_, operation)| operation.state == State::Vacant)
        else {
            return Err(Error::Capacity);
        };

        operation.reads = Some(reads);
        operation.writes = Some(writes);
        operation.state = State::Ready;
        Ok(OperationId::new(slot, operation.generation))
    }

    /// Returns the lifecycle state for a live operation identity.
    ///
    /// # Errors
    ///
    /// Returns an out-of-range or stale-operation error for an invalid
    /// identity.
    pub fn state(&self, operation: OperationId) -> Result<State, Error> {
        Ok(self.operation(operation)?.state)
    }

    /// Returns a committed resource value, never a private staged value.
    ///
    /// # Errors
    ///
    /// Returns an out-of-range or stale-resource error for an invalid
    /// identity.
    pub fn inspect(&self, resource: ResourceId) -> Result<T, Error> {
        Ok(self.resource(resource)?.value())
    }

    fn validate_declaration(&self, declaration: &[ResourceId]) -> Result<(), Error> {
        for (index, resource) in declaration.iter().enumerate() {
            self.resource(*resource)?;
            if declaration[..index].contains(resource) {
                return Err(Error::DuplicateDeclaration);
            }
        }
        Ok(())
    }

    fn resource(&self, identity: ResourceId) -> Result<&Resource<T>, Error> {
        let resource = self
            .resources
            .get(identity.slot())
            .ok_or(Error::ResourceOutOfRange)?;
        if resource.generation() != identity.generation() {
            return Err(Error::StaleResource);
        }
        Ok(resource)
    }

    fn operation(&self, identity: OperationId) -> Result<&Operation<'a>, Error> {
        let operation = self
            .operations
            .get(identity.slot())
            .ok_or(Error::OperationOutOfRange)?;
        if operation.generation != identity.generation() {
            return Err(Error::StaleOperation);
        }
        Ok(operation)
    }
}
