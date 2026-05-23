use clap::Parser;
use faster_paths::{
    contraction_hierarchy::ContractionHierarchy,
    hub_labeling::{HubLabeling, HubLabelingPathfinder},
};
use faster_paths_benchmarks::{DistanceType, benchmark_pathfinder};
use std::{fs::File, io::BufReader, path::PathBuf};

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

    benchmark_pathfinder(
        &mut pathfinder,
        contraction_hierarchy.num_vertices(),
        args.num,
    );
}
