use faster_paths::{
    graph::WeightedEdge, path::Query, pathfinder::ShortestPathFinder, types::Vertex,
    validation::generate_random_queries,
};
use graph_readers::{edges_from_dimacs, edges_from_fmi};
use num_traits::Zero;
use ordered_float::OrderedFloat;
use rayon::slice::ParallelSliceMut;
use std::{
    fs::File,
    io::BufReader,
    path::Path,
    time::{Duration, Instant},
};

pub type DistanceType = OrderedFloat<f64>;

pub fn load_graph_edges(graph: impl AsRef<Path>) -> Vec<WeightedEdge<DistanceType>> {
    let graph = graph.as_ref();
    let mut edges = match graph.extension().and_then(|e| e.to_str()) {
        Some("fmi") => edges_from_fmi(
            BufReader::new(File::open(graph).unwrap()),
            |vertex_parser| vertex_parser.parse::<Vertex>().ok(),
            |weight_parser| weight_parser.parse::<DistanceType>().ok(),
            |tail, head, weight| WeightedEdge { tail, head, weight },
        )
        .unwrap(),
        Some("gr") => edges_from_dimacs(
            BufReader::new(File::open(graph).unwrap()),
            |vertex_parser| vertex_parser.parse::<Vertex>().ok(),
            |weight_parser| weight_parser.parse::<DistanceType>().ok(),
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
}

pub fn benchmark<F>(
    warmup_queries: &[Query],
    benchmark_queries: &[Query],
    mut run_query: F,
) -> Duration
where
    F: FnMut(&Query),
{
    for query in warmup_queries {
        run_query(query);
    }

    let start = Instant::now();
    for query in benchmark_queries {
        run_query(query);
    }
    start.elapsed()
}

pub fn print_average(benchmark_target: &str, duration: Duration, num_queries: usize) {
    println!(
        "Took on average {:?} over {} {} queries.",
        duration / num_queries as u32,
        num_queries,
        benchmark_target,
    );
}

pub fn benchmark_pathfinder<P>(pathfinder: &mut P, num_vertices: usize, num_queries: usize)
where
    P: ShortestPathFinder,
{
    let warmup_queries = generate_random_queries(num_vertices, num_queries);
    let benchmark_queries = generate_random_queries(num_vertices, num_queries);

    let distance_duration = benchmark(&warmup_queries, &benchmark_queries, |query| {
        pathfinder.distance(query);
    });
    print_average("distance", distance_duration, benchmark_queries.len());

    let path_duration = benchmark(&warmup_queries, &benchmark_queries, |query| {
        pathfinder.path(query);
    });
    print_average("path", path_duration, benchmark_queries.len());
}

pub fn report_validation_result(
    validation_target: &str,
    tests_len: usize,
    validation_result: Result<Duration, Vec<String>>,
) -> bool {
    match validation_result {
        Ok(average_runtime) => {
            println!(
                "All {} {} correct. Average runtime: {:?}.",
                tests_len, validation_target, average_runtime
            );
            true
        }

        Err(failures) => {
            failures.iter().for_each(|message| eprintln!("{message}"));

            eprintln!(
                "{} of {} {} failed.",
                failures.len(),
                tests_len,
                validation_target
            );
            false
        }
    }
}
