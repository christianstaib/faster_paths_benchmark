use ch::{
    classical_search::DijkstraPathfinder,
    data_structures::VecSearchState,
    graph::{CsrGraph, GraphLike, WeightedEdge},
    path::{PathDistance, PathQuery},
    pathfinder::ShortestPathFinder,
    types::{Distance, VertexId},
};
use clap::Parser;
use graph_readers::{edges_from_dimacs, edges_from_fmi};
use indicatif::ParallelProgressIterator;
use ordered_float::OrderedFloat;
use rand::seq::index::sample;
use rayon::prelude::*;
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
) -> Vec<PathDistance<D>> {
    let num_vertices = graph.num_vertices();
    let mut rng = rand::rng();

    let queries = (0..num_tests)
        .map(|_| {
            let vertices = sample(&mut rng, num_vertices, 2);

            PathQuery {
                source: VertexId::new(vertices.index(0) as u32),
                target: VertexId::new(vertices.index(1) as u32),
            }
        })
        .collect::<Vec<_>>();

    queries
        .into_par_iter()
        .progress()
        .map_init(
            || DijkstraPathfinder::<_, VecSearchState<_>>::new(graph),
            |pathfinder, query| PathDistance::new(query, pathfinder.distance(&query)),
        )
        .collect::<Vec<_>>()
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

    let start = Instant::now();
    let tests = generate_tests(&graph, args.num_tests);
    println!("Took {:?}", start.elapsed());
    let output = File::create(&args.tests).unwrap();
    serde_json::to_writer_pretty(BufWriter::new(output), &tests).unwrap();

    println!("Wrote {} tests to {:?}.", tests.len(), args.tests);
}
