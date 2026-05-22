use faster_paths::{contraction_hierarchy::contract_graph_parallel, graph::WeightedEdge, types::Vertex};
use clap::Parser;
use faster_paths_benchmarks::DistanceType;
use graph_readers::{edges_from_dimacs, edges_from_fmi};
use num_traits::Zero;
use rayon::slice::ParallelSliceMut;
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
    time::Instant,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input graph file
    #[arg(short, long)]
    graph: PathBuf,

    /// Output CH file
    #[arg(short, long)]
    contraction_hierarchy: PathBuf,

    /// Accepted for CLI compatibility; faster_paths uses its built-in candidate fraction.
    #[arg(short, long = "fraction", default_value_t = 0.5)]
    _fraction: f64,
}

fn main() {
    let args = Args::parse();

    let edges = {
        let mut edges = match args.graph.extension().and_then(|e| e.to_str()) {
            Some("fmi") => edges_from_fmi(
                BufReader::new(File::open(&args.graph).unwrap()),
                |s| s.parse::<u32>().ok().map(Vertex::new),
                |s| s.parse::<DistanceType>().ok(),
                |tail, head, weight| WeightedEdge { tail, head, weight },
            )
            .unwrap(),
            Some("gr") => edges_from_dimacs(
                BufReader::new(File::open(&args.graph).unwrap()),
                |s| s.parse::<Vertex>().ok(),
                |s| s.parse::<DistanceType>().ok(),
                |tail, head, weight| WeightedEdge { tail, head, weight },
            )
            .unwrap(),
            Some(extension) => panic!("extension {} not found", extension),
            None => panic!("no extension found"),
        };
        edges.retain(|edge| edge.weight >= DistanceType::zero());
        edges.par_sort();
        edges.dedup_by_key(|edge| (edge.tail, edge.head));
        edges
    };
    let start = Instant::now();
    let contraction_hierarchy = contract_graph_parallel(&edges);
    println!("Contraction took {:?}", start.elapsed());

    let output = File::create(args.contraction_hierarchy).unwrap();
    postcard::to_io(&contraction_hierarchy, BufWriter::new(output)).unwrap();
}
