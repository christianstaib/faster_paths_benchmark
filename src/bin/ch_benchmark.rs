use ch::contraction_hierachy::{ContractionHierarchy, ContractionHierarchyPathfinder};
use ch::path::generate_queries;
use ch::pathfinder::ShortestPathFinder;
use clap::{Parser, ValueEnum};
use std::{fs::File, io::BufReader, path::PathBuf, time::Instant};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum BenchmarkMode {
    Distance,
    Path,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Contraction hierarchy file
    #[arg(short, long)]
    contraction_hierarchy: PathBuf,

    /// Test file
    #[arg(short, long)]
    num: usize,

    /// Benchmark mode
    #[arg(short, long, value_enum)]
    mode: BenchmarkMode,
}

type DistanceType = u32;

fn main() {
    let args = Args::parse();

    // Parse the ChGraph from the file
    let (contraction_hierarchy, _): (ContractionHierarchy<DistanceType>, _) = postcard::from_io((
        BufReader::new(File::open(&args.contraction_hierarchy).unwrap()),
        &mut [0; 1024],
    ))
    .unwrap();
    let mut pathfinder = ContractionHierarchyPathfinder::new(&contraction_hierarchy);

    let warmup_queries = generate_queries(contraction_hierarchy.num_vertices(), args.num);
    let benchmark_queries = generate_queries(contraction_hierarchy.num_vertices(), args.num);

    match args.mode {
        BenchmarkMode::Distance => warmup_queries.iter().for_each(|query| {
            pathfinder.distance(query);
        }),

        BenchmarkMode::Path => warmup_queries.iter().for_each(|query| {
            pathfinder.path(query);
        }),
    };

    let start = Instant::now();
    match args.mode {
        BenchmarkMode::Distance => benchmark_queries.iter().for_each(|query| {
            pathfinder.distance(query);
        }),

        BenchmarkMode::Path => benchmark_queries.iter().for_each(|query| {
            pathfinder.path(query);
        }),
    };
    let whole_duration = start.elapsed();

    println!(
        "Took on average {:?} over {} {:?} queries.",
        whole_duration / benchmark_queries.len() as u32,
        benchmark_queries.len(),
        args.mode,
    );
}
