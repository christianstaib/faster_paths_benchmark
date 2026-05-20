use ch::{
    contraction_hierachy::contract_graph_parallel,
    graph::{CsrGraph, WeightedEdge},
    types::VertexId,
};
use clap::Parser;
use graph_readers::edges_from_fmi;
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

type DistanceType = u32;

fn main() {
    let args = Args::parse();

    let edges = edges_from_fmi(
        BufReader::new(File::open(&args.graph).unwrap()),
        |s| s.parse::<u32>().ok().map(VertexId::new),
        |s| s.parse::<DistanceType>().ok(),
        |tail, head, weight| WeightedEdge { tail, head, weight },
    )
    .unwrap();
    let graph = CsrGraph::from_flat(edges);

    let contraction_hierarchy = contract_graph_parallel(&graph, args.fraction);
    let output = File::create(args.contraction_hierarchy).unwrap();

    postcard::to_io(&contraction_hierarchy, BufWriter::new(output)).unwrap();
}
