use clap::Parser;
use fast_paths::{FastGraph, InputGraph, PathCalculator};
use faster_paths::{
    classical_search::DijkstraPathfinder,
    data_structures::VecSearchState,
    graph::{CsrGraph, GraphLike, WeightedEdge},
    path::{Path, Query},
    pathfinder::ShortestPathFinder,
    types::Vertex,
    validation::{PathTestCase, generate_random_queries, validate_paths},
};
use graph_readers::{edges_from_dimacs, edges_from_fmi};
use indicatif::ParallelProgressIterator;
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

type DistanceType = usize;

struct FastPathsPathfinder<'a> {
    fast_graph: &'a FastGraph,
    path_calculator: PathCalculator,
}

impl<'a> FastPathsPathfinder<'a> {
    fn new(fast_graph: &'a FastGraph) -> Self {
        Self {
            fast_graph,
            path_calculator: fast_paths::create_calculator(fast_graph),
        }
    }
}

impl ShortestPathFinder for FastPathsPathfinder<'_> {
    type Distance = DistanceType;

    fn path(&mut self, query: &Query) -> Option<Path<Self::Distance>> {
        let shortest_path = self.path_calculator.calc_path(
            self.fast_graph,
            query.source.as_usize(),
            query.target.as_usize(),
        )?;

        Some(Path {
            vertices: shortest_path
                .get_nodes()
                .iter()
                .map(|&node| Vertex::from(u32::try_from(node).unwrap()))
                .collect(),
            distance: shortest_path.get_weight(),
        })
    }

    fn distance(&mut self, query: &Query) -> Option<Self::Distance> {
        self.path_calculator
            .calc_path(
                self.fast_graph,
                query.source.as_usize(),
                query.target.as_usize(),
            )
            .map(|path| path.get_weight())
    }
}

fn main() {
    let args = Args::parse();
    let edges = read_edges(&args.graph);
    let graph = CsrGraph::from_flat(edges.clone());
    let input_graph = build_fast_paths_input_graph(&edges);

    let num_validations = 100;
    let validation_queries = generate_random_queries(graph.num_vertices(), num_validations);
    let tests = validation_queries
        .into_par_iter()
        .progress()
        .map_init(
            || DijkstraPathfinder::<_, VecSearchState<_>>::new(&graph),
            |pathfinder, query| PathTestCase {
                query,
                distance: pathfinder.distance(&query),
            },
        )
        .collect::<Vec<_>>();

    let start = Instant::now();
    let fast_graph = fast_paths::prepare(&input_graph);
    println!(
        "full creation of fast_paths graph took {:?}",
        start.elapsed()
    );

    let mut pathfinder = FastPathsPathfinder::new(&fast_graph);

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
        let warmup = generate_random_queries(fast_graph.get_num_nodes(), benchmark_runs);
        let benchmark = generate_random_queries(fast_graph.get_num_nodes(), benchmark_runs);

        warmup.iter().for_each(|query| {
            pathfinder.path(query);
        });

        let start = Instant::now();
        benchmark.iter().for_each(|query| {
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
        let warmup = generate_random_queries(fast_graph.get_num_nodes(), benchmark_runs);
        let benchmark = generate_random_queries(fast_graph.get_num_nodes(), benchmark_runs);

        warmup.iter().for_each(|query| {
            pathfinder.distance(query);
        });

        let start = Instant::now();
        benchmark.iter().for_each(|query| {
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

fn read_edges(graph: &PathBuf) -> Vec<WeightedEdge<DistanceType>> {
    let mut edges = match graph.extension().and_then(|e| e.to_str()) {
        Some("fmi") => edges_from_fmi(
            BufReader::new(File::open(graph).unwrap()),
            |s| s.parse::<u32>().ok().map(Vertex::from),
            |s| s.parse::<DistanceType>().ok(),
            |tail, head, weight| WeightedEdge { tail, head, weight },
        )
        .unwrap(),
        Some("gr") => edges_from_dimacs(
            BufReader::new(File::open(graph).unwrap()),
            |s| s.parse::<Vertex>().ok(),
            |s| s.parse::<DistanceType>().ok(),
            |tail, head, weight| WeightedEdge { tail, head, weight },
        )
        .unwrap(),
        Some(extension) => panic!("extension {} not found", extension),
        None => panic!("no extension found"),
    };

    edges.retain(|edge| edge.weight > 0 && edge.tail != edge.head);
    edges.par_sort();
    edges.dedup_by_key(|edge| (edge.tail, edge.head));
    edges
}

fn build_fast_paths_input_graph(edges: &[WeightedEdge<DistanceType>]) -> InputGraph {
    let mut input_graph = InputGraph::new();

    for edge in edges {
        input_graph.add_edge(edge.tail.as_usize(), edge.head.as_usize(), edge.weight);
    }

    input_graph.freeze();
    input_graph
}
