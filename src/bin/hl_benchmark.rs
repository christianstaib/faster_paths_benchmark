use clap::Parser;
use faster_paths::{
    contraction_hierarchy::ContractionHierarchy,
    hub_labeling::{HubLabeling, HubLabelingPathfinder},
    path::Query,
    pathfinder::ShortestPathFinder,
    validation::generate_random_queries,
};
use faster_paths_benchmarks::DistanceType;
use std::{
    fs::File,
    io::BufReader,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Contraction hierarchy file
    #[arg(short, long)]
    contraction_hierarchy: PathBuf,

    /// Hub labeling file
    #[arg(short = 'l', long)]
    hub_labeling: PathBuf,

    /// Test file
    #[arg(short, long)]
    num: usize,
}

fn main() {
    let args = Args::parse();

    let (contraction_hierarchy, _): (ContractionHierarchy<DistanceType>, _) = postcard::from_io((
        BufReader::new(File::open(&args.contraction_hierarchy).unwrap()),
        &mut [0; 1024],
    ))
    .unwrap();

    let (hub_labeling, _): (HubLabeling<DistanceType>, _) = postcard::from_io((
        BufReader::new(File::open(&args.hub_labeling).unwrap()),
        &mut [0; 1024],
    ))
    .unwrap();

    let mut pathfinder = HubLabelingPathfinder::new(&contraction_hierarchy, &hub_labeling);

    let warmup_queries = generate_random_queries(contraction_hierarchy.num_vertices(), args.num);
    let benchmark_queries = generate_random_queries(contraction_hierarchy.num_vertices(), args.num);

    let distance_duration = benchmark(&warmup_queries, &benchmark_queries, |query| {
        pathfinder.distance(query);
    });
    print_average("distance", distance_duration, benchmark_queries.len());

    let path_duration = benchmark(&warmup_queries, &benchmark_queries, |query| {
        pathfinder.path(query);
    });
    print_average("path", path_duration, benchmark_queries.len());
}

fn benchmark<F>(warmup_queries: &[Query], benchmark_queries: &[Query], mut run_query: F) -> Duration
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

fn print_average(benchmark_target: &str, duration: Duration, num_queries: usize) {
    println!(
        "Took on average {:?} over {} {} queries.",
        duration / num_queries as u32,
        num_queries,
        benchmark_target,
    );
}
