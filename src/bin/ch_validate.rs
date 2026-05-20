use ch::contraction_hierachy::{ContractionHierarchy, ContractionHierarchyPathfinder};
use ch::graph::{CsrGraph, WeightedEdge};
use ch::path::PathDistance;
use ch::types::VertexId;
use ch::validation::validate;
use clap::Parser;
use graph_readers::edges_from_fmi;
use std::{fs::File, io::BufReader, path::PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input graph file
    #[arg(short, long)]
    graph: Option<PathBuf>,

    /// Test file
    #[arg(short, long)]
    tests: PathBuf,

    /// Contraction hierarchy file
    #[arg(short, long)]
    contraction_hierarchy: PathBuf,
}

type DistanceType = u32;

fn main() {
    let args = Args::parse();

    // Parse the ChGraph from the file
    let (contraction_hierarchy, _): (ContractionHierarchy<DistanceType>, _) = postcard::from_io((
        BufReader::new(File::open(&args.contraction_hierarchy).unwrap()),
        &mut [0; 1024],
    ))
    .unwrap();

    let graph = args.graph.as_ref().map(|graph| {
        let edges = edges_from_fmi(
            BufReader::new(File::open(graph).unwrap()),
            |s| s.parse::<u32>().ok().map(VertexId::new),
            |s| s.parse::<DistanceType>().ok(),
            |tail, head, weight| WeightedEdge { tail, head, weight },
        )
        .unwrap();

        CsrGraph::from_flat(edges)
    });
    let tests_input = File::open(&args.tests).unwrap();
    let tests: Vec<PathDistance<DistanceType>> =
        serde_json::from_reader(BufReader::new(tests_input)).unwrap();

    let mut pathfinder = ContractionHierarchyPathfinder::new(&contraction_hierarchy);
    let validation_target = if graph.is_some() {
        "paths"
    } else {
        "distances"
    };

    match validate(&tests, graph.as_ref(), &mut pathfinder) {
        Ok(average_runtime) => {
            println!(
                "All {} {} correct. Average runtime: {:?}.",
                tests.len(),
                validation_target,
                average_runtime
            );
        }

        Err(failures) => {
            failures.iter().for_each(|message| eprintln!("{message}"));

            eprintln!(
                "{} of {} {} failed.",
                failures.len(),
                tests.len(),
                validation_target
            );
            std::process::exit(1);
        }
    }
}
