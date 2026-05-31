use clap::Parser;
use faster_paths::{
    classical_search::DijkstraPathfinder,
    graph::{CsrGraph, GraphLike, WeightedEdge},
    path::Query,
    pathfinder::ShortestPathFinder,
    types::{Distance, Vertex},
    validation::PathTestCase,
};
use faster_paths_benchmarks::load_graph_edges;
use indicatif::ParallelProgressIterator;
use rand::seq::index::sample;
use rayon::prelude::*;
use std::{fs::File, io::BufWriter, path::PathBuf, time::Instant};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input graph file
    #[arg(short, long)]
    graph: PathBuf,

    /// Output tests file
    #[arg(short, long)]
    tests: PathBuf,

    /// Test count
    #[arg(short = 'n', long)]
    num_tests: usize,
}

fn generate_tests<D: Distance>(
    graph: &CsrGraph<WeightedEdge<D>>,
    num_tests: usize,
) -> Vec<PathTestCase<D>> {
    let num_vertices = graph.num_vertices();
    let mut rng = rand::rng();

    let queries = (0..num_tests)
        .map(|_| {
            let vertices = sample(&mut rng, num_vertices, 2);

            Query {
                source: Vertex::from(vertices.index(0) as u32),
                target: Vertex::from(vertices.index(1) as u32),
            }
        })
        .collect::<Vec<_>>();

    queries
        .into_par_iter()
        .progress()
        .map_init(
            || DijkstraPathfinder::new(graph),
            |pathfinder, query| PathTestCase {
                query,
                distance: pathfinder.distance(&query),
            },
        )
        .collect::<Vec<_>>()
}

fn main() {
    let args = Args::parse();

    let edges = load_graph_edges(&args.graph);
    let graph = CsrGraph::from_flat(edges);

    let start = Instant::now();
    let tests = generate_tests(&graph, args.num_tests);
    println!("Took {:?}", start.elapsed());
    let output = File::create(&args.tests).unwrap();
    serde_json::to_writer_pretty(BufWriter::new(output), &tests).unwrap();

    println!("Wrote {} tests to {:?}.", tests.len(), args.tests);
}
