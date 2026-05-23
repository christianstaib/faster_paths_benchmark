#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 /path/to/graph.{fmi,gr} [num_tests=1000] [epsilon=0] [dijkstra_queries=100] [ch_hl_queries=100000]"
  exit 1
fi

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

graph="$(realpath "$1")"
num_tests="${2:-1000}"
epsilon="${3:-0}"
dijkstra_benchmark_queries="${4:-100}"
ch_hl_benchmark_queries="${5:-100000}"

graph_dir="$(dirname "$graph")"
graph_name="$(basename "$graph")"
graph_stem="${graph_name%.*}"

tests_file="${graph_dir}/${graph_stem}_tests_${num_tests}.json"
ch_file="${graph_dir}/${graph_stem}.ch.postcard"
hl_file="${graph_dir}/${graph_stem}.hl.postcard"

run_bin() {
  local bin="$1"
  shift

  echo
  echo "==> ${bin}"
  "./target/release/${bin}" "$@"
}

echo "Graph:        $graph"
echo "Tests:        $tests_file"
echo "CH:           $ch_file"
echo "HL:           $hl_file"
echo "Num tests:    $num_tests"
echo "Epsilon:      $epsilon"
echo "Dijkstra n:   $dijkstra_benchmark_queries"
echo "CH/HL n:      $ch_hl_benchmark_queries"

echo
echo "==> cargo build -r"
cargo build -r --bins

run_bin generate_tests \
  --graph "$graph" \
  --tests "$tests_file" \
  --num-tests "$num_tests"

run_bin dijkstra_benchmark \
  --graph "$graph" \
  --num "$dijkstra_benchmark_queries"

run_bin ch_contract_parallel \
  --graph "$graph" \
  --contraction-hierarchy "$ch_file"

run_bin ch_validate \
  --graph "$graph" \
  --tests "$tests_file" \
  --contraction-hierarchy "$ch_file" \
  --epsilon "$epsilon"

run_bin ch_benchmark \
  --contraction-hierarchy "$ch_file" \
  --num "$ch_hl_benchmark_queries"

run_bin hl_merge \
  --contraction-hierarchy "$ch_file" \
  --hub-labeling "$hl_file" \
  --epsilon "$epsilon"

run_bin hl_validate \
  --graph "$graph" \
  --contraction-hierarchy "$ch_file" \
  --hub-labeling "$hl_file" \
  --tests "$tests_file" \
  --epsilon "$epsilon"

run_bin hl_benchmark \
  --contraction-hierarchy "$ch_file" \
  --hub-labeling "$hl_file" \
  --num "$ch_hl_benchmark_queries"

echo
echo "Done."
