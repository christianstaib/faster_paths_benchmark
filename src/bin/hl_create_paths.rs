use clap::Parser;
use faster_paths::{
    contraction_hierarchy::{
        ContractionHierarchy, build_working_graph, contract_working_graph_sequential_with_order,
    },
    graph::WeightedEdge,
    hub_labeling::{HubLabeling, HubLabelingPathfinder},
    pathfinder::ShortestPathFinder,
    types::Vertex,
    validation::generate_random_queries,
};
use faster_paths_benchmarks::DistanceType;
use graph_readers::{edges_from_dimacs, edges_from_fmi};
use indicatif::{ParallelProgressIterator, ProgressBar};
use num_traits::Zero;
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use std::{fs::File, io::BufReader, path::PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input graph file
    #[arg(short, long)]
    graph: PathBuf,

    /// Contraction hierarchy file
    #[arg(short, long)]
    contraction_hierarchy: PathBuf,

    /// Hub labeling File
    #[arg(short = 'l', long)]
    hub_labeling: PathBuf,
}

fn main() {
    let args = Args::parse();

    let edges = {
        let mut edges = match args.graph.extension().and_then(|e| e.to_str()) {
            Some("fmi") => edges_from_fmi(
                BufReader::new(File::open(&args.graph).unwrap()),
                |s| s.parse::<u32>().ok().map(Vertex::from),
                |s| s.parse::<DistanceType>().ok(),
                |tail, head, weight| WeightedEdge { tail, head, weight },
            )
            .unwrap(),
            Some("gr") => edges_from_dimacs(
                BufReader::new(File::open(&args.graph).unwrap()),
                |s| s.parse::<Vertex>().ok(),
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

    let n = 4_500_000;
    println!("Generation queries");
    let warmup_queries = generate_random_queries(contraction_hierarchy.num_vertices(), n);
    println!("Generation paths");
    let paths = warmup_queries
        .into_par_iter()
        .progress()
        .map_init(
            || HubLabelingPathfinder::new(&contraction_hierarchy, &hub_labeling),
            |pathfinder, query| pathfinder.path(&query),
        )
        .flatten()
        .map(|path| {
            path.vertices
                .into_iter()
                .map(|v| v.as_usize() as u32)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut hs = hitting_set(&paths);
    println!("{} hs has size", hs.len());

    let selected_vertices: HashSet<_> = hs.iter().copied().collect();
    let not_selected_vertices: HashSet<_> =
        (0..contraction_hierarchy.num_vertices() as u32).collect();

    let not_selected_vertices: HashSet<_> = not_selected_vertices
        .difference(&selected_vertices)
        .copied()
        .collect();

    hs.extend(not_selected_vertices);

    let order: Vec<Vertex> = hs.into_iter().map(Vertex::from).collect();
    std::mem::drop(paths);

    let working_graph = build_working_graph(&edges);
    let new_ch = contract_working_graph_sequential_with_order(working_graph, &order);
    let new_hl = HubLabeling::try_from_contraction_hierarchy(&new_ch, 0).unwrap();

    println!(
        "avg label size {}",
        new_hl.up_hub_labeling().num_flat() as f32 / contraction_hierarchy.num_vertices() as f32
    );
}

pub fn hitting_set(sets: &[Vec<u32>]) -> Vec<u32> {
    let mut active_sets: Vec<&Vec<u32>> = sets.iter().collect();
    let mut hitting_set = Vec::new();
    let progress_bar = ProgressBar::new(sets.len() as u64);

    while !active_sets.is_empty() {
        let previous_active_set_count = active_sets.len();
        let thread_count = rayon::current_num_threads().max(1);
        let chunk_size = active_sets.len().div_ceil(thread_count).max(1);

        let counts = active_sets
            .par_chunks(chunk_size)
            .map(|chunk| {
                let mut counts = FxHashMap::default();

                for set in chunk {
                    for &vertex in set.iter() {
                        *counts.entry(vertex).or_insert(0) += 1;
                    }
                }

                counts
            })
            .reduce(
                FxHashMap::default,
                |mut larger_counts, mut smaller_counts| {
                    if smaller_counts.len() > larger_counts.len() {
                        std::mem::swap(&mut larger_counts, &mut smaller_counts);
                    }

                    for (key, count) in smaller_counts {
                        *larger_counts.entry(key).or_insert(0) += count;
                    }

                    larger_counts
                },
            );

        let selected = counts
            .into_iter()
            .max_by(|(a_key, a_count), (b_key, b_count)| {
                a_count.cmp(b_count).then_with(|| b_key.cmp(a_key))
            })
            .map(|(key, _)| key)
            .expect("cannot compute a hitting set when an active set is empty");

        hitting_set.push(selected);

        active_sets = active_sets
            .into_par_iter()
            .filter(|set| !set.contains(&selected))
            .collect();

        progress_bar.inc((previous_active_set_count - active_sets.len()) as u64);
    }

    progress_bar.finish();

    hitting_set
}

#[cfg(test)]
mod tests {
    use super::hitting_set;

    #[test]
    fn hitting_set_returns_empty_for_no_sets() {
        assert!(hitting_set(&[]).is_empty());
    }

    #[test]
    fn hitting_set_covers_all_sets() {
        let sets = vec![vec![1, 2], vec![2, 3], vec![3, 4], vec![2, 4]];

        let result = hitting_set(&sets);

        assert_eq!(result, vec![2, 3]);
        assert!(
            sets.iter()
                .all(|set| result.iter().any(|vertex| set.contains(vertex)))
        );
    }

    #[test]
    #[should_panic(expected = "cannot compute a hitting set when an active set is empty")]
    fn hitting_set_panics_for_empty_sets() {
        hitting_set(&[vec![]]);
    }
}
