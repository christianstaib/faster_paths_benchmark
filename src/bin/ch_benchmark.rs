use clap::Parser;
use faster_paths::contraction_hierarchy::{ContractionHierarchy, ContractionHierarchyPathfinder};
use faster_paths_benchmarks::{DistanceType, benchmark_pathfinder};
use std::{fs::File, io::BufReader, path::PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Contraction hierarchy file
    #[arg(short, long)]
    contraction_hierarchy: PathBuf,

    /// Test file
    #[arg(short, long)]
    num: usize,
}

fn main() {
    let args = Args::parse();

    // Parse the ChGraph from the file
    let (contraction_hierarchy, _): (ContractionHierarchy<DistanceType>, _) = postcard::from_io((
        BufReader::new(File::open(&args.contraction_hierarchy).unwrap()),
        &mut [0; 1024],
    ))
    .unwrap();
    let mut pathfinder = ContractionHierarchyPathfinder::new(&contraction_hierarchy);

    benchmark_pathfinder(
        &mut pathfinder,
        contraction_hierarchy.num_vertices(),
        args.num,
    );
}
