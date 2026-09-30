//! Deterministic interleavings for terminal publication transitions.

use helium::{Error, Model, Operation, Resource, ResourceId, Stage, State};

fn id(slot: usize, generation: u64) -> ResourceId {
    ResourceId::new(slot, generation)
}

#[test]
fn abort_never_publishes_in_either_order() {
    for writer_first in [true, false] {
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

        if writer_first {
            model.admit(writer_op).expect("writer admits");
            model.stage(writer_op, x, 23).expect("write stages");
            assert_eq!(model.admit(reader), Err(Error::Conflict));
        } else {
            model.admit(reader).expect("reader admits");
            assert_eq!(model.admit(writer_op), Err(Error::Conflict));
        }

        model.abort(writer_op).expect("writer aborts");
        assert_eq!(model.inspect(x), Ok(11));
        assert_eq!(model.state(writer_op), Ok(State::Failed));
    }
}

#[test]
fn three_operation_terminal_orders_preserve_reservation_safety() {
    let orders = [
        [0_usize, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];

    for order in orders {
        for terminal_commit in [true, false] {
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
            let mut stages = [Stage::empty(), Stage::empty(), Stage::empty()];
            let mut model = Model::new(&mut resources, &mut operations, &mut stages);
            let ids = [id(0, 0), id(1, 0), id(2, 0)];
            let read_sets = [[ids[0]], [ids[1]], [ids[2]]];
            let write_sets = [[ids[1]], [ids[2]], [ids[0]]];
            let mut handles = [None, None, None];
            for index in order {
                handles[index] = Some(
                    model
                        .register(&read_sets[index], &write_sets[index])
                        .expect("operation registers"),
                );
            }

            for index in order {
                let _ = model.admit(handles[index].expect("operation handle"));
            }

            for left in 0..3 {
                for right in (left + 1)..3 {
                    if model.state(handles[left].expect("left handle")) == Ok(State::Running)
                        && model.state(handles[right].expect("right handle")) == Ok(State::Running)
                    {
                        assert!(!write_sets[left].iter().any(|resource| {
                            read_sets[right].contains(resource)
                                || write_sets[right].contains(resource)
                        }));
                        assert!(!write_sets[right].iter().any(|resource| {
                            read_sets[left].contains(resource)
                                || write_sets[left].contains(resource)
                        }));
                    }
                }
            }

            let terminal = handles[order[0]].expect("terminal handle");
            if terminal_commit {
                model.commit(terminal).expect("running operation commits");
            } else {
                model.abort(terminal).expect("running operation aborts");
            }
        }
    }
}

#[test]
fn empty_and_read_write_footprints_work_in_both_orders() {
    for reverse in [false, true] {
        let mut resources = [Resource::new(0, 11_u32)];
        let mut operations = [Operation::vacant(), Operation::vacant()];
        let mut stages = [Stage::empty()];
        let mut model = Model::new(&mut resources, &mut operations, &mut stages);
        let x = id(0, 0);
        let empty: [ResourceId; 0] = [];
        let same = [x];
        let mut handles = [None, None];
        for (index, handle) in handles.iter_mut().enumerate() {
            let operation = if (index == 0) ^ reverse {
                model.register(&empty, &empty).expect("empty registers")
            } else {
                model.register(&same, &same).expect("read-write registers")
            };
            *handle = Some(operation);
        }
        for operation in handles.into_iter().flatten() {
            model.admit(operation).expect("operation admits");
        }
        assert_eq!(
            model.state(handles[0].expect("empty handle")),
            Ok(State::Running)
        );
        assert_eq!(
            model.state(handles[1].expect("read-write handle")),
            Ok(State::Running)
        );
    }
}
