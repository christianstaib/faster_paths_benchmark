use faster_paths::{contraction_hierarchy::contract_graph_sequential, graph::WeightedEdge, types::Vertex};
use clap::Parser;
use faster_paths_benchmarks::DistanceType;
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
}

fn main() {
    let args = Args::parse();

    let edges = edges_from_fmi(
        BufReader::new(File::open(&args.graph).unwrap()),
        |s| s.parse::<u32>().ok().map(Vertex::new),
        |s| s.parse::<DistanceType>().ok(),
        |tail, head, weight| WeightedEdge { tail, head, weight },
    )
    .unwrap();
    let contraction_hierarchy = contract_graph_sequential(&edges);
    let output = File::create(args.contraction_hierarchy).unwrap();

    postcard::to_io(&contraction_hierarchy, BufWriter::new(output)).unwrap();
}
