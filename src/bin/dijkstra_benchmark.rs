use clap::Parser;
use faster_paths::{
    classical_search::DijkstraPathfinder,
    data_structures::VecSearchState,
    graph::{CsrGraph, GraphLike},
};
use faster_paths_benchmarks::{benchmark_pathfinder, load_graph_edges};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input graph file
    #[arg(short, long)]
    graph: PathBuf,

    /// Test file
    #[arg(short, long)]
    num: usize,
}

fn main() {
    let args = Args::parse();

    let edges = load_graph_edges(&args.graph);
    let graph = CsrGraph::from_flat(edges);
    let mut pathfinder = DijkstraPathfinder::<_, VecSearchState<_>>::new(&graph);

    benchmark_pathfinder(&mut pathfinder, graph.num_vertices(), args.num);
}
