use ch::{contraction_hierachy::ContractionHierarchy, hub_labeling::HubLabeling};
use clap::Parser;
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
    time::Instant,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Contraction hierarchy file
    #[arg(short, long)]
    contraction_hierarchy: PathBuf,

    /// Hub labeling File
    #[arg(short = 'l', long)]
    hub_labeling: PathBuf,

    /// Absolute comparison tolerance used while pruning labels
    #[arg(short, long)]
    epsilon: DistanceType,
}

type DistanceType = u32;

fn main() {
    let args = Args::parse();

    let (contraction_hierarchy, _): (ContractionHierarchy<DistanceType>, _) = postcard::from_io((
        BufReader::new(File::open(&args.contraction_hierarchy).unwrap()),
        &mut [0; 1024],
    ))
    .unwrap();

    let start = Instant::now();
    let hub_labeling =
        HubLabeling::try_from_contraction_hierarchy(&contraction_hierarchy, args.epsilon).unwrap();
    println!("Merging took {:?}", start.elapsed());

    let avg_label_size = hub_labeling.up_hub_labeling().num_flat() as f32
        / hub_labeling.up_hub_labeling().num_nested() as f32;
    println!("Average label size is {}", avg_label_size);

    let start = Instant::now();
    let hub_labeling_file = File::create(args.hub_labeling).unwrap();
    postcard::to_io(&hub_labeling, BufWriter::new(hub_labeling_file)).unwrap();
    println!("writing took {:?}", start.elapsed());
}
