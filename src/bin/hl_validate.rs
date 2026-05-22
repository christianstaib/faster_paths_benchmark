use faster_paths::{
    contraction_hierarchy::ContractionHierarchy,
    graph::WeightedEdge,
    hub_labeling::{HubLabeling, HubLabelingPathfinder},
    types::Vertex,
    validation::{PathTestCase, validate_distances, validate_paths},
};
use clap::Parser;
use faster_paths_benchmarks::DistanceType;
use graph_readers::edges_from_fmi;
use rayon::prelude::*;
use std::{fs::File, io::BufReader, path::PathBuf, time::Duration};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input graph file
    #[arg(short, long)]
    graph: PathBuf,

    /// Contraction hierarchy file
    #[arg(short, long)]
    contraction_hierarchy: PathBuf,

    /// Hub labeling File
    #[arg(short = 'l', long)]
    hub_labeling: PathBuf,

    /// Test file
    #[arg(short, long)]
    tests: PathBuf,

    /// Absolute comparison tolerance
    #[arg(short, long)]
    epsilon: DistanceType,
}

fn main() {
    let args = Args::parse();

    let mut graph_edges = edges_from_fmi(
        BufReader::new(File::open(&args.graph).unwrap()),
        |s| s.parse::<u32>().ok().map(Vertex::new),
        |s| s.parse::<DistanceType>().ok(),
        |tail, head, weight| WeightedEdge { tail, head, weight },
    )
    .unwrap();
    graph_edges.par_sort();
    graph_edges.dedup_by_key(|edge| (edge.tail, edge.head));

    let (contraction_hierarchy, _): (ContractionHierarchy<DistanceType>, _) = postcard::from_io((
        BufReader::new(File::open(&args.contraction_hierarchy).unwrap()),
        &mut [0; 1024],
    ))
    .unwrap();

    let (hub_labeling, _): (HubLabeling<DistanceType>, _) = postcard::from_io((
        BufReader::new(File::open(&args.hub_labeling).unwrap()),
        &mut [0; 1024],
    ))
    .unwrap();

    let tests_input = File::open(&args.tests).unwrap();
    let tests: Vec<PathTestCase<DistanceType>> =
        serde_json::from_reader(BufReader::new(tests_input)).unwrap();

    let mut pathfinder = HubLabelingPathfinder::new(&contraction_hierarchy, &hub_labeling);

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
