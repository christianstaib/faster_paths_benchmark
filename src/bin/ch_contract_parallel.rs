use clap::Parser;
use faster_paths::contraction_hierarchy::contract_graph_parallel;
use faster_paths_benchmarks::load_graph_edges;
use std::{fs::File, io::BufWriter, path::PathBuf, time::Instant};

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

    let edges = load_graph_edges(&args.graph);

    let start = Instant::now();
    let contraction_hierarchy = contract_graph_parallel(&edges);
    println!("Contraction took {:?}", start.elapsed());

    let output = File::create(args.contraction_hierarchy).unwrap();
    postcard::to_io(&contraction_hierarchy, BufWriter::new(output)).unwrap();
}
