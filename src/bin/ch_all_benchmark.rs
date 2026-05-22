use ch::{
    classical_search::DijkstraPathfinder,
    contraction_hierachy::{ContractionHierarchyPathfinder, contract_graph_parallel},
    data_structures::VecSearchState,
    graph::{CsrGraph, GraphLike, WeightedEdge},
    path::{PathDistance, generate_queries},
    pathfinder::ShortestPathFinder,
    types::VertexId,
    validation::validate_paths,
};
use clap::Parser;
use faster_paths_benchmarks::DistanceType;
use graph_readers::edges_from_dimacs;
use graph_readers::edges_from_fmi;
use indicatif::ParallelProgressIterator;
use num_traits::Zero;
use rayon::prelude::*;
use std::{fs::File, io::BufReader, path::PathBuf, time::Instant};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input graph file
    #[arg(short, long)]
    graph: PathBuf,

    /// Absolute comparison tolerance used during validation
    #[arg(short, long)]
    epsilon: DistanceType,
}

fn main() {
    let args = Args::parse();
    let edges = {
        let mut edges = match args.graph.extension().and_then(|e| e.to_str()) {
            Some("fmi") => edges_from_fmi(
                BufReader::new(File::open(&args.graph).unwrap()),
                |s| s.parse::<VertexId>().ok(),
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
        edges.retain(|edge| edge.weight >= DistanceType::zero());
        edges.par_sort();
        edges.dedup_by_key(|edge| (edge.tail, edge.head));
        edges
    };

    let graph = CsrGraph::from_flat(edges.clone());

    let num_valiations = 100;
    let validation_query = generate_queries(graph.num_vertices(), num_valiations);
    let tests = validation_query
        .into_par_iter()
        .progress()
        .map_init(
            || DijkstraPathfinder::<_, VecSearchState<_>>::new(&graph),
            |pathfinder, query| PathDistance::new(query, pathfinder.distance(&query)),
        )
        .collect::<Vec<_>>();

    let start = Instant::now();
    let contraction_hierarchy = contract_graph_parallel(&graph, 0.5);
    println!(
        "full creation of contraction_hierarchy took {:?}",
        start.elapsed()
    );

    let mut pathfinder = ContractionHierarchyPathfinder::new(&contraction_hierarchy);

    if let Err(failures) = validate_paths(&tests, &edges, &mut pathfinder, args.epsilon) {
        eprintln!(
            "Validation failed with {} of {} paths incorrect:",
            failures.len(),
            tests.len()
        );

        for (index, failure) in failures.iter().enumerate() {
            eprintln!("{:>4}. {failure}", index + 1);
        }

        std::process::exit(1);
    }

    println!("Validation ok");

    let benchmark_runs = 1_000;
    {
        let warmup = generate_queries(contraction_hierarchy.num_vertices(), benchmark_runs);
        let benchmark = generate_queries(contraction_hierarchy.num_vertices(), benchmark_runs);

        warmup.iter().for_each(|query| {
            pathfinder.path(query);
        });

        let start = Instant::now();
        warmup.iter().for_each(|query| {
            pathfinder.path(query);
        });
        let whole_duration = start.elapsed();
        println!(
            "Path: Took on average {:?} over {} queries.",
            whole_duration / benchmark.len() as u32,
            benchmark.len(),
        );
    }
    {
        let warmup = generate_queries(contraction_hierarchy.num_vertices(), benchmark_runs);
        let benchmark = generate_queries(contraction_hierarchy.num_vertices(), benchmark_runs);

        warmup.iter().for_each(|query| {
            pathfinder.distance(query);
        });

        let start = Instant::now();
        warmup.iter().for_each(|query| {
            pathfinder.distance(query);
        });
        let whole_duration = start.elapsed();
        println!(
            "Distance: Took on average {:?} over {} queries.",
            whole_duration / benchmark.len() as u32,
            benchmark.len(),
        );
    }
}
