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
    owner: Option<OperationId>,
    resource: Option<ResourceId>,
    value: Option<T>,
}

impl<T: Copy> Stage<T> {
    /// Creates an empty caller-owned staging slot.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            owner: None,
            resource: None,
            value: None,
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
    stages: &'a mut [Stage<T>],
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
            stages,
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

    /// Reads the committed value of a resource declared for reading.
    ///
    /// A staged replacement is intentionally not visible through this method;
    /// use [`Self::view`] to read an operation's private candidate instead.
    ///
    /// # Errors
    ///
    /// Returns an identity, lifecycle, or declaration error without exposing
    /// staged state.
    pub fn read(&self, operation: OperationId, resource: ResourceId) -> Result<T, Error> {
        let declared_read = {
            let operation = self.operation(operation)?;
            if operation.state != State::Running {
                return Err(Error::WrongState);
            }
            operation.reads.unwrap_or(&[]).contains(&resource)
        };
        let resource = self.resource(resource)?;
        if !declared_read {
            return Err(Error::UndeclaredRead);
        }
        Ok(resource.value())
    }

    /// Reads the value visible to an operation, including its own staged value.
    ///
    /// A write-only resource has no readable committed value, but becomes
    /// visible through this method after the operation stages its replacement.
    /// Other operations never see a private staged value.
    ///
    /// # Errors
    ///
    /// Returns an identity, lifecycle, or declaration error without changing
    /// any state.
    pub fn view(&self, operation: OperationId, resource: ResourceId) -> Result<T, Error> {
        let (declared_read, declared_write) = {
            let operation = self.operation(operation)?;
            if operation.state != State::Running {
                return Err(Error::WrongState);
            }
            let reads = operation.reads.unwrap_or(&[]);
            let writes = operation.writes.unwrap_or(&[]);
            (reads.contains(&resource), writes.contains(&resource))
        };
        let resource_value = self.resource(resource)?.value();
        if !declared_read && !declared_write {
            return Err(Error::UndeclaredRead);
        }
        if declared_write {
            for stage in self.stages.iter() {
                if stage.owner == Some(operation)
                    && stage.resource == Some(resource)
                    && let Some(value) = stage.value
                {
                    return Ok(value);
                }
            }
        }
        if declared_read {
            return Ok(resource_value);
        }
        Err(Error::UndeclaredRead)
    }

    /// Stages a private replacement for a declared write resource.
    ///
    /// Restaging replaces the operation's existing candidate in place, so it
    /// does not require another caller-provided staging slot. A new candidate
    /// consumes one empty slot and has no committed effect.
    ///
    /// # Errors
    ///
    /// Returns an identity, lifecycle, declaration, or capacity error without
    /// changing any existing stage.
    pub fn stage(
        &mut self,
        operation: OperationId,
        resource: ResourceId,
        value: T,
    ) -> Result<(), Error> {
        let declared_write = {
            let operation_state = self.operation(operation)?;
            if operation_state.state != State::Running {
                return Err(Error::WrongState);
            }
            operation_state.writes.unwrap_or(&[]).contains(&resource)
        };
        self.resource(resource)?;
        if !declared_write {
            return Err(Error::UndeclaredWrite);
        }

        for stage in self.stages.iter_mut() {
            if stage.owner == Some(operation) && stage.resource == Some(resource) {
                stage.value = Some(value);
                return Ok(());
            }
        }
        let Some(stage) = self.stages.iter_mut().find(|stage| stage.owner.is_none()) else {
            return Err(Error::Capacity);
        };
        stage.owner = Some(operation);
        stage.resource = Some(resource);
        stage.value = Some(value);
        Ok(())
    }

    /// Admits a ready operation when its complete footprint is compatible
    /// with every currently running operation.
    ///
    /// A reservation is derived from the complete declaration. A conflict
    /// therefore leaves the operation ready and does not reserve any part of
    /// its footprint.
    ///
    /// # Errors
    ///
    /// Returns an operation identity, lifecycle, resource identity, or
    /// admission conflict error without changing the candidate state unless
    /// admission succeeds.
    pub fn admit(&mut self, operation: OperationId) -> Result<(), Error> {
        let (reads, writes) = {
            let candidate = self.operation(operation)?;
            if candidate.state != State::Ready {
                return Err(Error::WrongState);
            }
            (
                candidate.reads.unwrap_or(&[]),
                candidate.writes.unwrap_or(&[]),
            )
        };

        self.validate_declaration(reads)?;
        self.validate_declaration(writes)?;

        for running in &self.operations[..] {
            if running.state != State::Running {
                continue;
            }
            let running_reads = running.reads.unwrap_or(&[]);
            let running_writes = running.writes.unwrap_or(&[]);
            self.validate_declaration(running_reads)?;
            self.validate_declaration(running_writes)?;
            if Self::overlaps(writes, running_writes)
                || Self::overlaps(writes, running_reads)
                || Self::overlaps(reads, running_writes)
            {
                return Err(Error::Conflict);
            }
        }

        self.operation_mut(operation)?.state = State::Running;
        Ok(())
    }

    /// Publishes all private writes for a running operation and releases its
    /// complete reservation.
    ///
    /// Every stage is checked before the first committed value is changed.
    /// A successful commit clears the operation's stages and makes its
    /// terminal state visible as [`State::Committed`].
    ///
    /// # Errors
    ///
    /// Returns an identity or lifecycle error, or a declaration error for a
    /// corrupted stage, without publishing a partial result.
    pub fn commit(&mut self, operation: OperationId) -> Result<(), Error> {
        let (writes, state) = {
            let operation = self.operation(operation)?;
            (operation.writes.unwrap_or(&[]), operation.state)
        };
        if state != State::Running {
            return Err(Error::WrongState);
        }

        for stage in self.stages.iter() {
            if stage.owner != Some(operation) {
                continue;
            }
            let Some(resource) = stage.resource else {
                return Err(Error::UndeclaredWrite);
            };
            if stage.value.is_none() {
                return Err(Error::UndeclaredWrite);
            }
            if !writes.contains(&resource) {
                return Err(Error::UndeclaredWrite);
            }
            self.resource(resource)?;
        }

        for stage in self.stages.iter() {
            if stage.owner != Some(operation) {
                continue;
            }
            if let (Some(resource), Some(value)) = (stage.resource, stage.value) {
                self.resources[resource.slot()].set_value(value);
            }
        }
        self.clear_stages(operation);
        self.operation_mut(operation)?.state = State::Committed;
        Ok(())
    }

    /// Discards an operation's private writes and releases its reservation.
    ///
    /// Ready operations may be aborted before admission; running operations
    /// release their complete reservation. The terminal state is
    /// [`State::Failed`] until explicit recycling.
    ///
    /// # Errors
    ///
    /// Returns an identity or lifecycle error without changing the operation.
    pub fn abort(&mut self, operation: OperationId) -> Result<(), Error> {
        let state = self.operation(operation)?.state;
        if state != State::Ready && state != State::Running {
            return Err(Error::WrongState);
        }
        self.clear_stages(operation);
        self.operation_mut(operation)?.state = State::Failed;
        Ok(())
    }

    /// Recycles a committed or failed terminal operation slot.
    ///
    /// Recycling clears its declaration and advances the operation
    /// generation, making prior identities stale. Generation overflow is
    /// rejected without changing the terminal operation.
    ///
    /// # Errors
    ///
    /// Returns an identity, lifecycle, or generation-overflow error without
    /// changing the slot on failure.
    pub fn recycle(&mut self, operation: OperationId) -> Result<(), Error> {
        let (state, generation) = {
            let operation_slot = self.operation(operation)?;
            (operation_slot.state, operation_slot.generation)
        };
        if state != State::Committed && state != State::Failed {
            return Err(Error::WrongState);
        }
        if self
            .stages
            .iter()
            .any(|stage| stage.owner == Some(operation))
        {
            return Err(Error::WrongState);
        }
        let generation = generation.checked_add(1).ok_or(Error::GenerationOverflow)?;
        let operation_slot = self.operation_mut(operation)?;
        operation_slot.reads = None;
        operation_slot.writes = None;
        operation_slot.generation = generation;
        operation_slot.state = State::Vacant;
        Ok(())
    }

    fn clear_stages(&mut self, operation: OperationId) {
        for stage in self.stages.iter_mut() {
            if stage.owner == Some(operation) {
                *stage = Stage::empty();
            }
        }
    }

    fn overlaps(left: &[ResourceId], right: &[ResourceId]) -> bool {
        left.iter()
            .any(|resource| right.iter().any(|other| resource.slot() == other.slot()))
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

    fn operation_mut(&mut self, identity: OperationId) -> Result<&mut Operation<'a>, Error> {
        let operation = self
            .operations
            .get_mut(identity.slot())
            .ok_or(Error::OperationOutOfRange)?;
        if operation.generation != identity.generation() {
            return Err(Error::StaleOperation);
        }
        Ok(operation)
    }
}
