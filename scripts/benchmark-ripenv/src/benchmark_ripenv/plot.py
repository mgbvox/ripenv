"""Plot benchmark results from hyperfine JSON exports."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.ticker as ticker


def load_results(json_path: Path) -> dict:
    """Load a hyperfine JSON result file."""
    with json_path.open() as f:
        return json.load(f)


def plot_benchmark_comparison(
    result_dir: Path,
    output_path: Path | None = None,
) -> None:
    """Generate a grouped bar chart comparing tools across all benchmarks.

    Reads all ``*.json`` files in *result_dir* produced by ``hyperfine --export-json``.
    """
    json_files = sorted(result_dir.glob("*.json"))
    if not json_files:
        print(f"No JSON result files found in {result_dir}", file=sys.stderr)
        sys.exit(1)

    # Collect data: {benchmark_name: {command_name: {mean, stddev}}}.
    benchmarks: dict[str, dict[str, dict[str, float]]] = {}
    for json_file in json_files:
        benchmark_name = json_file.stem
        data = load_results(json_file)
        benchmarks[benchmark_name] = {}
        for result in data["results"]:
            # Extract tool name (strip fixture label in parens).
            command_name = result["command"]
            benchmarks[benchmark_name][command_name] = {
                "mean": result["mean"],
                "stddev": result["stddev"],
            }

    # Determine unique tool names across all benchmarks.
    all_tools: list[str] = []
    for bench_data in benchmarks.values():
        for tool_name in bench_data:
            if tool_name not in all_tools:
                all_tools.append(tool_name)

    # Assign colors per tool category.
    color_map: dict[str, str] = {}
    palette = {
        "ripenv": "#2196F3",
        "pipenv": "#FF9800",
        "uv": "#4CAF50",
    }
    for tool_name in all_tools:
        for key, color in palette.items():
            if key in tool_name.lower():
                color_map[tool_name] = color
                break
        else:
            color_map[tool_name] = "#9E9E9E"

    benchmark_names = list(benchmarks.keys())
    num_benchmarks = len(benchmark_names)
    num_tools = len(all_tools)

    fig, ax = plt.subplots(figsize=(max(10, num_benchmarks * 1.5), 6))

    bar_width = 0.8 / num_tools
    x_positions = range(num_benchmarks)

    for tool_index, tool_name in enumerate(all_tools):
        means = []
        stddevs = []
        for bench_name in benchmark_names:
            bench_data = benchmarks[bench_name]
            if tool_name in bench_data:
                means.append(bench_data[tool_name]["mean"])
                stddevs.append(bench_data[tool_name]["stddev"])
            else:
                means.append(0)
                stddevs.append(0)

        x_offset = [x + tool_index * bar_width for x in x_positions]
        ax.bar(
            x_offset,
            means,
            bar_width,
            yerr=stddevs,
            label=tool_name,
            color=color_map[tool_name],
            capsize=3,
            edgecolor="white",
            linewidth=0.5,
        )

    # Formatting.
    ax.set_xlabel("Benchmark")
    ax.set_ylabel("Time (seconds)")
    ax.set_title("ripenv vs pipenv vs uv — Benchmark Comparison")
    ax.set_xticks([x + bar_width * (num_tools - 1) / 2 for x in x_positions])
    ax.set_xticklabels(benchmark_names, rotation=30, ha="right")
    ax.yaxis.set_major_formatter(ticker.FormatStrFormatter("%.2fs"))
    ax.legend(loc="upper left")
    ax.grid(axis="y", alpha=0.3)

    fig.tight_layout()

    if output_path is None:
        output_path = result_dir / "benchmark-results.png"

    fig.savefig(output_path, dpi=150)
    print(f"Plot saved to {output_path}")
    plt.close(fig)


def main() -> None:
    """CLI entry point for plotting benchmark results."""
    parser = argparse.ArgumentParser(
        prog="bench-ripenv-plot",
        description="Plot hyperfine benchmark results",
    )
    parser.add_argument(
        "result_dir",
        type=Path,
        help="Directory containing hyperfine JSON result files",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="Output image path (default: result_dir/benchmark-results.png)",
    )

    args = parser.parse_args()
    plot_benchmark_comparison(args.result_dir, args.output)


if __name__ == "__main__":
    main()
