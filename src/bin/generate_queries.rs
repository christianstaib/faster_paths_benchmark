use ch::{
    graph::{CsrGraph, GraphLike, WeightedEdge},
    types::VertexId,
    validation::generate_queries,
};
use clap::Parser;
use graph_readers::edges_from_fmi;
use ordered_float::OrderedFloat;
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

    /// Query count
    #[arg(short, long)]
    n: usize,

    /// Output query file
    #[arg(short, long)]
    out: PathBuf,
}

type DistanceType = OrderedFloat<f32>;

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

    let queries = generate_queries(graph.num_vertices(), args.n);
    let output = File::create(&args.out).unwrap();
    serde_json::to_writer_pretty(BufWriter::new(output), &queries).unwrap();

    println!("Wrote {} queries to {:?}.", queries.len(), args.out);
}
