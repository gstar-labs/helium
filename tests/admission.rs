//! External tests for complete reservation admission.

use helium::{Error, Model, Operation, OperationId, Resource, ResourceId, State};

fn ids<const N: usize>(slots: [usize; N]) -> [ResourceId; N] {
    slots.map(|slot| ResourceId::new(slot, 0))
}

#[test]
fn shared_reads_disjoint_writes_run_together() {
    let mut resources = [
        Resource::new(0, 10_u32),
        Resource::new(0, 20_u32),
        Resource::new(0, 30_u32),
    ];
    let mut operations = [Operation::vacant(), Operation::vacant()];
    let mut stages = [];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let shared_read = ids([0]);
    let writes_a = ids([1]);
    let writes_b = ids([2]);

    let operation_a = model
        .register(&shared_read, &writes_a)
        .expect("operation A registers");
    let operation_b = model
        .register(&shared_read, &writes_b)
        .expect("operation B registers");

    model.admit(operation_a).expect("operation A admits");
    model
        .admit(operation_b)
        .expect("disjoint writes admit together");
    assert_eq!(model.state(operation_a), Ok(State::Running));
    assert_eq!(model.state(operation_b), Ok(State::Running));
}

#[test]
fn cross_read_write_conflicts() {
    let mut resources = [Resource::new(0, 10_u32), Resource::new(0, 20_u32)];
    let mut operations = [Operation::vacant(), Operation::vacant()];
    let mut stages = [];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let reads = ids([0, 1]);
    let write_x = ids([0]);
    let write_y = ids([1]);

    let operation_a = model
        .register(&reads, &write_x)
        .expect("operation A registers");
    let operation_b = model
        .register(&reads, &write_y)
        .expect("operation B registers");

    model.admit(operation_a).expect("operation A admits");
    assert_eq!(model.admit(operation_b), Err(Error::Conflict));
    assert_eq!(model.state(operation_b), Ok(State::Ready));
}

#[test]
fn read_read_is_allowed_but_write_conflicts_with_read_and_write() {
    let mut resources = [Resource::new(0, 10_u32)];
    let mut operations = [
        Operation::vacant(),
        Operation::vacant(),
        Operation::vacant(),
        Operation::vacant(),
    ];
    let mut stages = [];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let resource = ids([0]);
    let empty: [ResourceId; 0] = [];

    let reader_a = model.register(&resource, &empty).expect("reader A");
    let reader_b = model.register(&resource, &empty).expect("reader B");
    let writer_a = model.register(&empty, &resource).expect("writer A");
    let writer_b = model.register(&empty, &resource).expect("writer B");

    model.admit(reader_a).expect("reader A admits");
    model.admit(reader_b).expect("shared readers admit");
    assert_eq!(model.admit(writer_a), Err(Error::Conflict));
    assert_eq!(model.state(writer_a), Ok(State::Ready));

    assert_eq!(model.admit(reader_a), Err(Error::WrongState));
    assert_eq!(model.admit(writer_b), Err(Error::Conflict));
    assert_eq!(model.state(writer_b), Ok(State::Ready));
}

#[test]
fn blocked_last_member_does_not_hold_first_member() {
    let mut resources = [
        Resource::new(0, 10_u32),
        Resource::new(0, 20_u32),
        Resource::new(0, 30_u32),
    ];
    let mut operations = [
        Operation::vacant(),
        Operation::vacant(),
        Operation::vacant(),
    ];
    let mut stages = [];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let empty: [ResourceId; 0] = [];
    let slot_zero = ids([0]);
    let slot_two = ids([2]);
    let candidate_writes = ids([0, 2]);

    let running_writer = model
        .register(&empty, &slot_two)
        .expect("running writer registers");
    let candidate = model
        .register(&empty, &candidate_writes)
        .expect("candidate registers");
    let independent = model
        .register(&empty, &slot_zero)
        .expect("independent writer registers");

    model.admit(running_writer).expect("running writer admits");
    assert_eq!(model.admit(candidate), Err(Error::Conflict));
    assert_eq!(model.state(candidate), Ok(State::Ready));
    model
        .admit(independent)
        .expect("slot zero was not partially reserved");
    assert_eq!(model.state(independent), Ok(State::Running));
}

#[test]
fn empty_footprint_is_admitted() {
    let mut resources = [Resource::new(0, 10_u32)];
    let mut operations = [Operation::vacant()];
    let mut stages = [];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let empty: [ResourceId; 0] = [];
    let operation = model.register(&empty, &empty).expect("empty operation");

    model.admit(operation).expect("empty footprint admits");
    assert_eq!(model.state(operation), Ok(State::Running));
}

#[test]
fn stale_operation_and_wrong_state_are_rejected() {
    let mut resources = [Resource::new(0, 10_u32)];
    let mut operations = [Operation::vacant()];
    let mut stages = [];
    let mut model = Model::new(&mut resources, &mut operations, &mut stages);
    let empty: [ResourceId; 0] = [];
    let operation = model.register(&empty, &empty).expect("operation");

    assert_eq!(
        model.admit(OperationId::new(1, 0)),
        Err(Error::OperationOutOfRange)
    );
    assert_eq!(
        model.admit(OperationId::new(0, 1)),
        Err(Error::StaleOperation)
    );
    model.admit(operation).expect("ready operation admits");
    assert_eq!(model.admit(operation), Err(Error::WrongState));
}
