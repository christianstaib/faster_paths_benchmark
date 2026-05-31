use clap::Parser;
use faster_paths::{
    contraction_hierarchy::ContractionHierarchy,
    hub_labeling::{HubLabeling, HubLabelingPathfinder},
    validation::{PathTestCase, validate_distances, validate_paths},
};
use faster_paths_benchmarks::{DistanceType, load_graph_edges, report_validation_result};
use std::{fs::File, io::BufReader, path::PathBuf};

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

    let graph_edges = load_graph_edges(&args.graph);
 
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
