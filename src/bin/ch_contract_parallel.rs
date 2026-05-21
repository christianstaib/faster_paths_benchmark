use ch::{
    contraction_hierachy::contract_graph_parallel,
    graph::{CsrGraph, WeightedEdge},
    types::VertexId,
};
use clap::Parser;
use graph_readers::{edges_from_dimacs, edges_from_fmi};
use ordered_float::OrderedFloat;
use rayon::slice::ParallelSliceMut;
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
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

    /// Candidate fraction
    #[arg(short, long, default_value_t = 0.5)]
    fraction: f64,
}

type DistanceType = OrderedFloat<f64>;

fn main() {
    let args = Args::parse();

    let edges = {
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
    let graph = CsrGraph::from_flat(edges);

    let contraction_hierarchy = contract_graph_parallel(&graph, args.fraction);
    let output = File::create(args.contraction_hierarchy).unwrap();

    postcard::to_io(&contraction_hierarchy, BufWriter::new(output)).unwrap();
}
