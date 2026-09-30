//! Checked identity and bounded registration behavior.

use helium::{Error, Model, Operation, OperationId, Resource, ResourceId, Stage, State};

#[test]
fn registers_valid_footprint() {
    let mut resources = [Resource::new(7, 11_u32), Resource::new(9, 29_u32)];
    let mut operations = [Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);

    let first = ResourceId::new(0, 7);
    let second = ResourceId::new(1, 9);
    let reads = [first];
    let writes = [second];
    let operation = model.register(&reads, &writes).expect("valid declaration");

    assert_eq!(operation, OperationId::new(0, 0));
    assert_eq!(model.state(operation), Ok(State::Ready));
    assert_eq!(model.inspect(first), Ok(11));
    assert_eq!(model.inspect(second), Ok(29));
}

#[test]
fn rejects_invalid_declaration_atomically() {
    let mut resources = [Resource::new(7, 11_u32), Resource::new(9, 29_u32)];
    let mut operations = [Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);

    let valid = ResourceId::new(0, 7);
    let stale = ResourceId::new(1, 8);
    let invalid_reads = [valid];
    let invalid_writes = [stale];
    assert_eq!(
        model.register(&invalid_reads, &invalid_writes),
        Err(Error::StaleResource)
    );

    let reads = [valid];
    let operation = model
        .register(&reads, &[])
        .expect("rejected registration leaves slot vacant");
    assert_eq!(operation, OperationId::new(0, 0));
    assert_eq!(model.state(operation), Ok(State::Ready));
}

#[test]
fn duplicate_within_set_is_rejected_but_read_write_overlap_is_valid() {
    let mut resources = [Resource::new(7, 11_u32)];
    let mut operations = [Operation::vacant(), Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let resource = ResourceId::new(0, 7);

    let duplicate_reads = [resource, resource];
    assert_eq!(
        model.register(&duplicate_reads, &[]),
        Err(Error::DuplicateDeclaration)
    );
    let reads = [resource];
    let writes = [resource];
    let operation = model
        .register(&reads, &writes)
        .expect("read/write overlap is legal");
    assert_eq!(model.state(operation), Ok(State::Ready));
}

#[test]
fn operation_capacity_and_out_of_range_are_explicit() {
    let mut resources = [Resource::new(7, 11_u32)];
    let mut operations = [Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let resource = ResourceId::new(0, 7);
    let writes = [resource];
    let empty_reads: [ResourceId; 0] = [];

    let first = model
        .register(&empty_reads, &writes)
        .expect("first operation");
    assert_eq!(first, OperationId::new(0, 0));
    assert_eq!(model.register(&empty_reads, &writes), Err(Error::Capacity));
    let out_of_range = [ResourceId::new(1, 7)];
    assert_eq!(
        model.register(&empty_reads, &out_of_range),
        Err(Error::ResourceOutOfRange)
    );
    assert_eq!(
        model.state(OperationId::new(1, 0)),
        Err(Error::OperationOutOfRange)
    );
}
