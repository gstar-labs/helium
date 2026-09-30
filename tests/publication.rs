//! External tests for checked reads and private staged writes.

use helium::{Error, Model, Operation, Resource, ResourceId, Stage, State};

fn id(slot: usize, generation: u64) -> ResourceId {
    ResourceId::new(slot, generation)
}

#[test]
fn staging_is_private_and_read_your_writes() {
    let mut resources = [Resource::new(0, 11_u32)];
    let mut operations = [Operation::vacant(), Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let x = id(0, 0);
    let reads = [x];
    let writes = [x];

    let operation = model
        .register(&reads, &writes)
        .expect("operation registers");
    let competing = model.register(&reads, &[]).expect("reader registers");
    model.admit(operation).expect("operation admits");
    assert_eq!(model.admit(competing), Err(Error::Conflict));

    assert_eq!(model.read(operation, x), Ok(11));
    model.stage(operation, x, 23).expect("write stages");
    assert_eq!(model.inspect(x), Ok(11));
    assert_eq!(model.read(operation, x), Ok(11));
    assert_eq!(model.view(operation, x), Ok(23));
}

#[test]
fn write_only_cannot_read_old_value() {
    let mut resources = [Resource::new(0, 11_u32)];
    let mut operations = [Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let x = id(0, 0);
    let empty: [ResourceId; 0] = [];
    let writes = [x];
    let operation = model.register(&empty, &writes).expect("writer registers");
    model.admit(operation).expect("writer admits");

    assert_eq!(model.read(operation, x), Err(Error::UndeclaredRead));
    assert_eq!(model.view(operation, x), Err(Error::UndeclaredRead));
    model.stage(operation, x, 23).expect("write stages");
    assert_eq!(model.view(operation, x), Ok(23));
    assert_eq!(model.read(operation, x), Err(Error::UndeclaredRead));
}

#[test]
fn unauthorized_access_is_rejected() {
    let mut resources = [Resource::new(7, 11_u32), Resource::new(0, 29_u32)];
    let mut operations = [Operation::vacant(), Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let x = id(0, 7);
    let y = id(1, 0);
    let reads = [x];
    let writes = [y];
    let operation = model
        .register(&reads, &writes)
        .expect("operation registers");
    let ready_operation = model
        .register(&reads, &writes)
        .expect("second operation registers");
    model.admit(operation).expect("operation admits");

    assert_eq!(model.read(operation, id(0, 8)), Err(Error::StaleResource));
    assert_eq!(
        model.stage(operation, id(1, 1), 31),
        Err(Error::StaleResource)
    );
    assert_eq!(model.read(ready_operation, y), Err(Error::WrongState));
    assert_eq!(model.view(ready_operation, y), Err(Error::WrongState));
    assert_eq!(model.stage(ready_operation, y, 31), Err(Error::WrongState));
    assert_eq!(model.read(operation, y), Err(Error::UndeclaredRead));
    assert_eq!(model.stage(operation, x, 31), Err(Error::UndeclaredWrite));
}

#[test]
fn full_stage_buffer_allows_replacement() {
    let mut resources = [Resource::new(0, 11_u32), Resource::new(0, 17_u32)];
    let mut operations = [Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let x = id(0, 0);
    let y = id(1, 0);
    let empty: [ResourceId; 0] = [];
    let writes = [x, y];
    let operation = model.register(&empty, &writes).expect("writer registers");
    model.admit(operation).expect("writer admits");

    model.stage(operation, x, 23).expect("first stage");
    assert_eq!(model.view(operation, x), Ok(23));
    model
        .stage(operation, x, 29)
        .expect("restage replaces in place");
    assert_eq!(model.view(operation, x), Ok(29));
    assert_eq!(model.stage(operation, y, 31), Err(Error::Capacity));
    assert_eq!(model.view(operation, x), Ok(29));
    assert_eq!(model.inspect(x), Ok(11));
}

#[test]
fn zero_stage_capacity_preserves_committed_value() {
    let mut resources = [Resource::new(0, 11_u32)];
    let mut operations = [Operation::vacant()];
    let mut stages: [Stage<u32>; 0] = [];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let x = id(0, 0);
    let empty: [ResourceId; 0] = [];
    let writes = [x];
    let operation = model.register(&empty, &writes).expect("writer registers");
    model.admit(operation).expect("writer admits");

    assert_eq!(model.stage(operation, x, 23), Err(Error::Capacity));
    assert_eq!(model.inspect(x), Ok(11));
    assert_eq!(model.state(operation), Ok(State::Running));
}
