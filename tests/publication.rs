//! External tests for checked reads and private staged writes.

use helium::{Error, Model, Operation, OperationId, Resource, ResourceId, Stage, State};

fn id(slot: usize, generation: u64) -> ResourceId {
    ResourceId::new(slot, generation)
}

static EMPTY: [ResourceId; 0] = [];
static WRITE_X: [ResourceId; 1] = [ResourceId::new(0, 0)];

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

#[test]
fn multi_resource_commit_publishes_both_then_releases() {
    let mut resources = [Resource::new(0, 11_u32), Resource::new(0, 17_u32)];
    let mut operations = [Operation::vacant(), Operation::vacant()];
    let mut stages = [Stage::empty(), Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let x = id(0, 0);
    let y = id(1, 0);
    let empty: [ResourceId; 0] = [];
    let reads = [x];
    let write_set = [x, y];
    let writer_op = model
        .register(&empty, &write_set)
        .expect("writer registers");
    let reader = model.register(&reads, &empty).expect("reader registers");
    model.admit(writer_op).expect("writer admits");
    assert_eq!(model.admit(reader), Err(Error::Conflict));

    model.stage(writer_op, x, 23).expect("x stages");
    model.stage(writer_op, y, 29).expect("y stages");
    model.commit(writer_op).expect("writer commits");

    assert_eq!(model.inspect(x), Ok(23));
    assert_eq!(model.inspect(y), Ok(29));
    assert_eq!(model.state(writer_op), Ok(State::Committed));
    model.admit(reader).expect("reader admits after commit");
}

#[test]
fn abort_discards_stages_and_releases() {
    let mut resources = [Resource::new(0, 11_u32)];
    let mut operations = [Operation::vacant(), Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let x = id(0, 0);
    let empty: [ResourceId; 0] = [];
    let reads = [x];
    let write_set = [x];
    let writer_op = model
        .register(&empty, &write_set)
        .expect("writer registers");
    let reader = model.register(&reads, &empty).expect("reader registers");
    model.admit(writer_op).expect("writer admits");
    assert_eq!(model.admit(reader), Err(Error::Conflict));

    model.stage(writer_op, x, 23).expect("write stages");
    model.abort(writer_op).expect("writer aborts");

    assert_eq!(model.inspect(x), Ok(11));
    assert_eq!(model.state(writer_op), Ok(State::Failed));
    model.admit(reader).expect("reader admits after abort");
}

#[test]
fn constructing_a_model_hides_stages_from_a_previous_model() {
    let mut stages = [Stage::empty()];
    {
        let mut resources = [Resource::new(0, 11_u32)];
        let mut operations = [Operation::vacant()];
        let x = id(0, 0);
        let mut old_model = Model::new(&mut resources, &mut operations, &mut stages);
        let old_operation = old_model
            .register(&EMPTY, &WRITE_X)
            .expect("old operation registers");
        old_model
            .admit(old_operation)
            .expect("old operation admits");
        old_model
            .stage(old_operation, x, 23)
            .expect("old write stages");
    }

    let mut resources = [Resource::new(0, 11_u32)];
    let mut operations = [Operation::vacant()];
    let x = id(0, 0);
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let operation = model
        .register(&EMPTY, &WRITE_X)
        .expect("new operation registers");
    model.admit(operation).expect("new operation admits");

    assert_eq!(model.view(operation, x), Err(Error::UndeclaredRead));
    assert_eq!(model.inspect(x), Ok(11));
}

#[test]
fn constructing_a_model_drops_stages_before_unstaged_commit() {
    let mut stages = [Stage::empty()];
    {
        let mut resources = [Resource::new(0, 11_u32)];
        let mut operations = [Operation::vacant()];
        let x = id(0, 0);
        let mut old_model = Model::new(&mut resources, &mut operations, &mut stages);
        let old_operation = old_model
            .register(&EMPTY, &WRITE_X)
            .expect("old operation registers");
        old_model
            .admit(old_operation)
            .expect("old operation admits");
        old_model
            .stage(old_operation, x, 23)
            .expect("old write stages");
    }

    let mut resources = [Resource::new(0, 11_u32)];
    let mut operations = [Operation::vacant()];
    let x = id(0, 0);
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let operation = model
        .register(&EMPTY, &WRITE_X)
        .expect("new operation registers");
    model.admit(operation).expect("new operation admits");
    model.commit(operation).expect("new operation commits");

    assert_eq!(model.inspect(x), Ok(11));
}

#[test]
fn terminal_reuse_cannot_replay_private_writes() {
    let mut resources = [Resource::new(0, 11_u32)];
    let mut operations = [Operation::vacant()];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let x = id(0, 0);
    let empty: [ResourceId; 0] = [];
    let writes = [x];
    let old = model
        .register(&empty, &writes)
        .expect("operation registers");
    model.admit(old).expect("operation admits");
    model.stage(old, x, 23).expect("write stages");
    model.abort(old).expect("operation aborts");
    model.recycle(old).expect("operation recycles");

    assert_eq!(model.state(old), Err(Error::StaleOperation));
    let replacement = model
        .register(&empty, &writes)
        .expect("replacement registers");
    assert_eq!(replacement, OperationId::new(0, 1));
    model.admit(replacement).expect("replacement admits");
    assert_eq!(model.inspect(x), Ok(11));
}

#[test]
fn recycle_overflow_rejects_without_mutation() {
    let mut resources = [Resource::new(0, 11_u32)];
    let mut operations = [Operation::vacant_with_generation(u64::MAX)];
    let mut stages: [Stage<u32>; 0] = [];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let empty: [ResourceId; 0] = [];
    let operation = model.register(&empty, &empty).expect("operation registers");
    model.abort(operation).expect("operation aborts");

    assert_eq!(model.recycle(operation), Err(Error::GenerationOverflow));
    assert_eq!(model.state(operation), Ok(State::Failed));
    assert_eq!(model.register(&empty, &empty), Err(Error::Capacity));
}

#[test]
fn commit_invalid_state_does_not_change_lifecycle() {
    let mut resources = [Resource::new(0, 11_u32)];
    let mut operations = [
        Operation::vacant(),
        Operation::vacant(),
        Operation::vacant(),
    ];
    let mut stages = [Stage::empty()];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let x = id(0, 0);
    let empty: [ResourceId; 0] = [];
    let writes = [x];
    let committed = model.register(&empty, &writes).expect("writer registers");
    let ready = model.register(&empty, &empty).expect("ready registers");
    let failed = model.register(&empty, &empty).expect("failed registers");
    model.admit(committed).expect("writer admits");
    model.stage(committed, x, 23).expect("write stages");
    model.commit(committed).expect("writer commits");
    assert_eq!(model.commit(committed), Err(Error::WrongState));
    assert_eq!(model.abort(committed), Err(Error::WrongState));
    assert_eq!(model.abort(ready), Ok(()));
    assert_eq!(model.abort(ready), Err(Error::WrongState));
    assert_eq!(model.commit(failed), Err(Error::WrongState));
    assert_eq!(model.inspect(x), Ok(23));
}
