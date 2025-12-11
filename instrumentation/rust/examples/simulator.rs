use std::fmt::{Debug, Display};

use petgraph::{Directed, Direction, Graph, graph::NodeIndex, visit::EdgeRef};
use quent::{ExporterOptions, OperatorObserver, PlanObserver};
use quent_events::{
    coordinator,
    engine::{self, EngineImplementationAttributes},
    operator, plan, query, worker,
};

use rand::{Rng, rng};
use tracing::info;
use uuid::Uuid;

fn initialize_tracing() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();
}

struct Operator<T: Debug> {
    id: Uuid,
    parents: Vec<Uuid>,
    kind: T,
}

impl<T> Operator<T>
where
    T: Debug,
{
    fn name(&self) -> String {
        format!("{:?}", self.kind)
    }

    fn new(kind: T, parents: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::now_v7(),
            parents,
            kind,
        }
    }
}

impl<T> Display for Operator<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug)]
struct Port {
    id: Uuid,
    name: &'static str,
}

#[derive(Debug)]
struct Edge {
    source: Port,
    target: Port,
}

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Edge {
    fn new(source: &'static str, target: &'static str) -> Edge {
        Edge {
            source: Port {
                id: Uuid::now_v7(),
                name: source,
            },
            target: Port {
                id: Uuid::now_v7(),
                name: target,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Logical {
    Scan,
    Project,
    Join,
    Sort,
    Limit,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Physical {
    FileSystemScan,
    JoinPartition,
    JoinLocal,
    Sort,
    Limit,
    Output,
}

type LogicalOp = Operator<Logical>;
type LogicalPlan = Graph<LogicalOp, Edge, Directed>;

type PhysicalOp = Operator<Physical>;
type PhysicalPlan = Graph<PhysicalOp, Edge, Directed>;

// Create the following logical plan:
// Scan -> Project \
//                  -> Join -> Sort -> Limit -> Output
// Scan -> Project /
fn logical_plan() -> LogicalPlan {
    // Add a scan --> project branch and return the (project, project output port) Uuids.
    fn add_scan_project_branch(plan: &mut LogicalPlan) -> NodeIndex {
        let scan = plan.add_node(LogicalOp::new(Logical::Scan, vec![]));
        let project = plan.add_node(LogicalOp::new(Logical::Project, vec![]));
        plan.add_edge(scan, project, Edge::new("out", "in"));

        project
    }

    let mut plan = Graph::new();

    let project_a = add_scan_project_branch(&mut plan);
    let project_b = add_scan_project_branch(&mut plan);

    let join = plan.add_node(LogicalOp::new(Logical::Join, vec![]));
    plan.add_edge(project_a, join, Edge::new("out", "left"));
    plan.add_edge(project_b, join, Edge::new("out", "right"));

    let sort = plan.add_node(LogicalOp::new(Logical::Sort, vec![]));
    plan.add_edge(join, sort, Edge::new("out", "in"));

    let limit = plan.add_node(LogicalOp::new(Logical::Limit, vec![]));
    plan.add_edge(sort, limit, Edge::new("out", "in"));

    let output = plan.add_node(LogicalOp::new(Logical::Output, vec![]));
    plan.add_edge(limit, output, Edge::new("out", "in"));

    plan
}

fn make_physical_plan(logical: &LogicalPlan) -> PhysicalPlan {
    // Find the output node
    let output = logical
        .node_indices()
        .collect::<Vec<_>>()
        .into_iter()
        .find(|n| logical[*n].kind == Logical::Output)
        .unwrap();

    // Build a physical plan
    let mut physical = PhysicalPlan::new();

    lower_logical(logical, &mut physical, output, None);

    physical
}

fn lower_logical(
    logical: &LogicalPlan,
    physical: &mut PhysicalPlan,
    logical_current_idx: NodeIndex,
    physical_target_idx_port: Option<(NodeIndex, &'static str)>,
) {
    let current_logical_op = &logical[logical_current_idx];

    match current_logical_op.kind {
        Logical::Scan => {
            unimplemented!("this shouldn't happen in this simulator, yet")
        }
        Logical::Project => {
            // Check whether this project has an incoming scan source to simulate predicate pushdown
            if let Some(scan_edge) = logical
                .edges_directed(logical_current_idx, Direction::Incoming)
                .find(|edge| logical[edge.source()].kind == Logical::Scan)
            {
                let scan_op = &logical[scan_edge.source()];
                let source = physical.add_node(PhysicalOp::new(
                    Physical::FileSystemScan,
                    vec![current_logical_op.id, scan_op.id],
                ));
                if let Some((target_node, target_port)) = physical_target_idx_port {
                    physical.add_edge(source, target_node, Edge::new(target_port, "in"));
                }
            } else {
                unimplemented!("this shouldn't happen in this simulator, yet");
            }
        }
        Logical::Join => {
            // split up in a partition stage and join stage
            let partition = physical.add_node(PhysicalOp::new(
                Physical::JoinPartition,
                vec![current_logical_op.id],
            ));
            let local = physical.add_node(PhysicalOp::new(
                Physical::JoinLocal,
                vec![current_logical_op.id],
            ));
            physical.add_edge(partition, local, Edge::new("build_out", "build_in"));
            physical.add_edge(partition, local, Edge::new("probe_out", "probe_in"));

            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical.add_edge(local, target_node, Edge::new("out", target_port));
            }

            // Recurse up both branches
            for input_edge in logical.edges_directed(logical_current_idx, Direction::Incoming) {
                lower_logical(
                    logical,
                    physical,
                    input_edge.source(),
                    Some((partition, input_edge.weight().target.name)),
                );
            }
        }
        Logical::Sort => {
            let sort =
                physical.add_node(PhysicalOp::new(Physical::Sort, vec![current_logical_op.id]));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical.add_edge(sort, target_node, Edge::new("out", target_port));
            }
            let input_edge = logical
                .edges_directed(logical_current_idx, Direction::Incoming)
                .next()
                .unwrap();
            lower_logical(
                logical,
                physical,
                input_edge.source(),
                Some((sort, input_edge.weight().target.name)),
            );
        }
        Logical::Limit => {
            let limit = physical.add_node(PhysicalOp::new(
                Physical::Limit,
                vec![current_logical_op.id],
            ));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical.add_edge(limit, target_node, Edge::new("out", target_port));
            }
            let input_edge = logical
                .edges_directed(logical_current_idx, Direction::Incoming)
                .next()
                .unwrap();
            lower_logical(
                logical,
                physical,
                input_edge.source(),
                Some((limit, input_edge.weight().target.name)),
            );
        }
        Logical::Output => {
            let output = physical.add_node(PhysicalOp::new(
                Physical::Output,
                vec![current_logical_op.id],
            ));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical.add_edge(output, target_node, Edge::new("out", target_port));
            }
            let input_edge = logical
                .edges_directed(logical_current_idx, Direction::Incoming)
                .next()
                .unwrap();
            lower_logical(
                logical,
                physical,
                input_edge.source(),
                Some((output, input_edge.weight().target.name)),
            );
        }
    }
}

fn create_plan_events<T>(
    query_id: Uuid,
    plan_obs: &PlanObserver,
    op_obs: &OperatorObserver,
    plan: &Graph<Operator<T>, Edge, Directed>,
) -> Uuid
where
    T: Debug,
{
    let plan_id = Uuid::now_v7();

    plan_obs.init(
        plan_id,
        plan::Init {
            query_id,
            worker_id: None,
            parent_id: None,
            edges: plan
                .edge_references()
                .map(|edge| (edge.weight().source.id, edge.weight().target.id))
                .collect(),
        },
    );

    plan_obs.executing(plan_id, Default::default());

    // Nonsensically create all operator events.
    let nodes = petgraph::algo::toposort(plan, None).unwrap();
    info!(
        "Topological order: {:?}",
        nodes
            .iter()
            .map(|node| format!("{:?}: {:?}", node, plan[node.clone()].kind))
            .collect::<Vec<_>>()
    );

    for node_idx in nodes.into_iter() {
        let op = &plan[node_idx];
        op_obs.init(
            op.id,
            operator::Init {
                plan_id,
                parent_plan_ids: op.parents.clone(),
                name: Some(op.name()),
                ports: plan
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                    .map(|edge| operator::Port {
                        id: edge.weight().target.id,
                        is_input: true,
                        name: Some(edge.weight().target.name.to_string()),
                    })
                    .chain(
                        plan.edges_directed(node_idx, petgraph::Direction::Outgoing)
                            .map(|edge| operator::Port {
                                id: edge.weight().source.id,
                                is_input: false,
                                name: Some(edge.weight().source.name.to_string()),
                            }),
                    )
                    .collect(),
            },
        );

        op_obs.waiting_for_inputs(op.id, Default::default());
        op_obs.executing(op.id, Default::default());
        op_obs.blocked(op.id, Default::default());
        op_obs.executing(op.id, Default::default());
        op_obs.waiting_for_inputs(op.id, Default::default());
        op_obs.finalizing(op.id, Default::default());
        op_obs.exit(op.id, Default::default());
    }

    plan_obs.idle(plan_id, Default::default());
    plan_obs.finalizing(plan_id, Default::default());
    plan_obs.exit(plan_id, Default::default());

    plan_id
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    let engine_id = Uuid::now_v7();

    info!("simulating engine - http://localhost:8080/analyzer/engine/{engine_id}");

    let context =
        quent::Context::try_new(ExporterOptions::Collector(Default::default()), engine_id)?;

    info!("context created, creating events...");

    // Spawn engine
    let engine_obs = context.engine_observer();
    engine_obs.init(
        engine_id,
        engine::Init {
            name: Some(format!("holodeck-{:04x}", rng().random::<u32>())),
            implementation: Some(EngineImplementationAttributes {
                name: Some("Simulator".into()),
                version: Some("0.0.0-PoC".into()),
            }),
        },
    );

    // Spawn workers
    let worker_obs = context.worker_observer();
    let worker_ids = std::iter::repeat_with(|| Uuid::now_v7())
        .take(4)
        .collect::<Vec<_>>();
    for (worker_index, worker_id) in worker_ids.iter().enumerate() {
        worker_obs.init(
            *worker_id,
            worker::Init {
                engine_id,
                name: Some(format!("worker-{worker_index}")),
            },
        );
    }
    for worker_id in worker_ids.iter() {
        worker_obs.operating(*worker_id, worker::Operating {});
    }

    engine_obs.operating(engine_id, engine::Operating {});

    let coordinator_futures: Vec<_> = std::iter::repeat_with(|| Uuid::now_v7())
        .take(2)
        .map(|coordinator_id| {
            info!("simulating coordinator - http://localhost:8080/analyzer/engine/{engine_id}/coordinator/{coordinator_id}");
            std::thread::spawn({
                let engine_id = engine_id.clone();
                let coordinator_obs = context.coordinator_observer();
                let query_obs = context.query_observer();
                let plan_obs = context.plan_observer();
                let operator_obs = context.operator_observer();

                move || {
                    coordinator_obs.init(
                        coordinator_id,
                        coordinator::Init {
                            engine_id,
                            name: Some(format!("coordinator-{:04x}", rng().random::<u32>())),
                        },
                    );
                    coordinator_obs.operating(coordinator_id, coordinator::Operating {});

                    let query_futures: Vec<_> = std::iter::repeat_with(|| Uuid::now_v7())
                        .take(2)
                        .map(|query_id| {
                            std::thread::spawn({
                                let query_obs = query_obs.clone();
                                let plan_obs = plan_obs.clone();
                                let operator_obs = operator_obs.clone();
                                move || {
                                    info!("simulating query - http://localhost:8080/analyzer/engine/{engine_id}/query/{query_id}");
                                    query_obs.init(
                                        query_id,
                                        query::Init {
                                            coordinator_id,
                                            name: Some(format!("query-{}", rng().random::<u32>())),
                                        },
                                    );
                                    query_obs.planning(query_id, query::Planning {});
                                    let l_plan = logical_plan();
                                    let p_plan = make_physical_plan(&l_plan);
                                    query_obs.executing(query_id, query::Executing {});

                                    create_plan_events(query_id, &plan_obs, &operator_obs, &l_plan);
                                    // TODO(johanpel): properly nest this
                                    create_plan_events(query_id, &plan_obs, &operator_obs, &p_plan);

                                    query_obs.idle(query_id, query::Idle {});
                                    query_obs.finalizing(query_id, query::Finalizing {});
                                    query_obs.exit(query_id, query::Exit {});
                                }
                            })
                        })
                        .collect();

                    for query_future in query_futures {
                        query_future.join().unwrap();
                    }

                    coordinator_obs.finalizing(coordinator_id, coordinator::Finalizing {});
                    coordinator_obs.exit(coordinator_id, coordinator::Exit {});
                }
            })
        })
        .collect();

    for coordinator_future in coordinator_futures {
        coordinator_future.join().unwrap();
    }

    engine_obs.finalizing(engine_id, engine::Finalizing {});

    // Shut down workers.
    for worker_id in worker_ids.iter() {
        worker_obs.finalizing(*worker_id, worker::Finalizing {});
        worker_obs.exit(*worker_id, worker::Exit {});
    }

    engine_obs.exit(engine_id, engine::Exit {});

    info!("events pushed, waiting 1s to flush (for now :tm:)");
    // TODO(johanpel): ensure the channels are flushed on drop
    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}
