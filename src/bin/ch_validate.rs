use ch::contraction_hierachy::{ContractionHierarchy, ContractionHierarchyPathfinder};
use ch::graph::WeightedEdge;
use ch::path::PathDistance;
use ch::types::VertexId;
use ch::validation::{validate_distances, validate_paths};
use clap::Parser;
use graph_readers::{edges_from_dimacs, edges_from_fmi};
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::{fs::File, io::BufReader, path::PathBuf, time::Duration};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input graph file
    #[arg(short, long)]
    graph: PathBuf,

    /// Test file
    #[arg(short, long)]
    tests: PathBuf,

    /// Contraction hierarchy file
    #[arg(short, long)]
    contraction_hierarchy: PathBuf,

    /// Absolute comparison tolerance
    #[arg(short, long)]
    epsilon: DistanceType,
}

type DistanceType = OrderedFloat<f64>;

fn main() {
    let args = Args::parse();

    // Parse the ChGraph from the file
    let (contraction_hierarchy, _): (ContractionHierarchy<DistanceType>, _) = postcard::from_io((
        BufReader::new(File::open(&args.contraction_hierarchy).unwrap()),
        &mut [0; 1024],
    ))
    .unwrap();

    let graph_edges = {
        let mut edges = match args.graph.extension().and_then(|e| e.to_str()) {
            Some("fmi") => edges_from_fmi(
                BufReader::new(File::open(&args.graph).unwrap()),
                |s| s.parse::<u32>().ok().map(VertexId::new),
                |s| s.parse::<DistanceType>().ok(),
                |tail, head, weight| WeightedEdge { tail, head, weight },
            )
            .unwrap(),
            Some("gr") => edges_from_dimacs(
                BufReader::new(File::open(&args.graph).unwrap()),
                |s| s.parse::<VertexId>().ok(),
                |s| s.parse::<DistanceType>().ok(),
                |tail, head, weight| WeightedEdge { tail, head, weight },
            )
            .unwrap(),
            Some(extension) => panic!("extension {} not found", extension),
            None => panic!("no extension found"),
        };
        edges.retain(|edge| edge.weight.is_sign_positive());
        edges.par_sort();
        edges.dedup_by_key(|edge| (edge.tail, edge.head));
        edges
    };
    let tests_input = File::open(&args.tests).unwrap();
    let tests: Vec<PathDistance<DistanceType>> =
        serde_json::from_reader(BufReader::new(tests_input)).unwrap();

    let mut pathfinder = ContractionHierarchyPathfinder::new(&contraction_hierarchy);

    let distance_result = validate_distances(&tests, &mut pathfinder, args.epsilon);
    let path_result = validate_paths(&tests, &graph_edges, &mut pathfinder, args.epsilon);

    let distances_valid = report_validation_result("distances", tests.len(), distance_result);
    let paths_valid = report_validation_result("paths", tests.len(), path_result);

    if !distances_valid || !paths_valid {
        std::process::exit(1);
    }
}

fn report_validation_result(
    validation_target: &str,
    tests_len: usize,
    validation_result: Result<Duration, Vec<String>>,
) -> bool {
    match validation_result {
        Ok(average_runtime) => {
            println!(
                "All {} {} correct. Average runtime: {:?}.",
                tests_len, validation_target, average_runtime
            );
            true
        }

        Err(failures) => {
            failures.iter().for_each(|message| eprintln!("{message}"));

            eprintln!(
                "{} of {} {} failed.",
                failures.len(),
                tests_len,
                validation_target
            );
            false
        }
    }
}
