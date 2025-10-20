#!/usr/bin/env python3
"""
CodeHUD Dependency Graph Visualizer

Creates actual graphical dependency visualizations using NetworkX and matplotlib.
This demonstrates real chart/graph output vs text-based visualizations.
"""

import sys
import json
from pathlib import Path
from typing import Dict, List, Tuple, Any

# Set matplotlib backend before importing pyplot
import matplotlib
matplotlib.use('Agg')  # Use non-interactive backend
import matplotlib.pyplot as plt
import networkx as nx
import numpy as np

def load_analysis_data(analysis_file: str) -> Dict[str, Any]:
    """Load analysis data from CodeHUD export"""
    try:
        with open(analysis_file, 'r') as f:
            data = json.load(f)
        return data
    except Exception as e:
        print(f"Error loading analysis data: {e}")
        return {}

def extract_dependencies(analysis_data: Dict[str, Any]) -> List[Tuple[str, str]]:
    """Extract dependency relationships from analysis data"""
    dependencies = []

    # Look for dependency data in different possible locations
    content = analysis_data.get('content', {})

    # Check Dependencies section
    deps_section = content.get('Dependencies', {})
    if deps_section:
        # Look for dependency graph data
        dep_graph = deps_section.get('dependency_graph', {})
        nodes = dep_graph.get('nodes', [])
        edges = dep_graph.get('edges', [])

        # Add edges as dependencies
        for edge in edges:
            if isinstance(edge, list) and len(edge) >= 2:
                dependencies.append((edge[0], edge[1]))

        # Look for coupling metrics
        coupling_metrics = deps_section.get('coupling_metrics', [])
        for metric in coupling_metrics:
            if isinstance(metric, dict) and 'from' in metric and 'to' in metric:
                dependencies.append((metric['from'], metric['to']))

    # Check Topology section for additional dependencies
    topology = content.get('Topology', {})
    if topology:
        coupling_metrics = topology.get('coupling_metrics', [])
        for metric in coupling_metrics:
            if isinstance(metric, dict) and 'from' in metric and 'to' in metric:
                dependencies.append((metric['from'], metric['to']))

    return dependencies

def create_dependency_graph(dependencies: List[Tuple[str, str]], analysis_data: Dict[str, Any]) -> nx.DiGraph:
    """Create NetworkX directed graph from dependencies"""
    G = nx.DiGraph()

    # Add all nodes and edges
    for source, target in dependencies:
        G.add_edge(source, target)

    # If no dependencies found, create a sample graph for demonstration
    if len(dependencies) == 0:
        print("No dependencies found in data, creating sample graph...")
        sample_deps = [
            ("main.py", "utils.py"),
            ("main.py", "config.py"),
            ("utils.py", "helpers.py"),
            ("config.py", "validators.py"),
            ("api.py", "models.py"),
            ("api.py", "utils.py"),
            ("models.py", "database.py"),
            ("database.py", "config.py"),
            ("tests.py", "main.py"),
            ("tests.py", "api.py")
        ]
        for source, target in sample_deps:
            G.add_edge(source, target)

    return G

def calculate_node_importance(G: nx.DiGraph) -> Dict[str, float]:
    """Calculate importance metrics for nodes"""
    # Use multiple centrality measures
    try:
        pagerank = nx.pagerank(G)
        in_degree = dict(G.in_degree())
        out_degree = dict(G.out_degree())

        # Combine metrics (normalize to 0-1)
        max_in = max(in_degree.values()) if in_degree.values() else 1
        max_out = max(out_degree.values()) if out_degree.values() else 1

        importance = {}
        for node in G.nodes():
            # Weighted combination of metrics
            score = (
                pagerank.get(node, 0) * 0.4 +
                (in_degree.get(node, 0) / max_in) * 0.3 +
                (out_degree.get(node, 0) / max_out) * 0.3
            )
            importance[node] = score

        return importance
    except:
        # Fallback: use degree centrality
        return nx.degree_centrality(G)

def visualize_dependency_graph(G: nx.DiGraph, output_file: str = "dependency_graph.png"):
    """Create and save dependency graph visualization"""

    plt.figure(figsize=(16, 12))
    plt.title("CodeHUD Dependency Graph Visualization", fontsize=16, fontweight='bold')

    # Calculate layout
    if len(G.nodes()) > 50:
        # Use faster layout for large graphs
        pos = nx.spring_layout(G, k=3, iterations=20)
    else:
        # Use better layout for smaller graphs
        pos = nx.spring_layout(G, k=2, iterations=50)

    # Calculate node importance and sizes
    importance = calculate_node_importance(G)
    node_sizes = [max(300, importance.get(node, 0) * 2000) for node in G.nodes()]

    # Color nodes by importance
    node_colors = [importance.get(node, 0) for node in G.nodes()]

    # Draw nodes
    nodes = nx.draw_networkx_nodes(
        G, pos,
        node_size=node_sizes,
        node_color=node_colors,
        cmap=plt.cm.Reds,
        alpha=0.8,
        edgecolors='black',
        linewidths=1
    )

    # Draw edges with varying thickness based on importance
    edge_weights = []
    for source, target in G.edges():
        # Edge weight based on target node importance
        weight = importance.get(target, 0) * 3 + 0.5
        edge_weights.append(weight)

    nx.draw_networkx_edges(
        G, pos,
        width=edge_weights,
        alpha=0.6,
        edge_color='gray',
        arrowsize=20,
        arrowstyle='->'
    )

    # Add labels (simplified for readability)
    labels = {}
    for node in G.nodes():
        # Simplify file names
        if '/' in node:
            label = Path(node).name
        else:
            label = node

        # Truncate long names
        if len(label) > 12:
            label = label[:10] + '..'
        labels[node] = label

    nx.draw_networkx_labels(
        G, pos,
        labels=labels,
        font_size=8,
        font_weight='bold'
    )

    # Add statistics text
    stats_text = f"""Graph Statistics:
Nodes: {G.number_of_nodes()}
Edges: {G.number_of_edges()}
Density: {nx.density(G):.3f}
Avg In-Degree: {sum(dict(G.in_degree()).values()) / G.number_of_nodes():.1f}"""

    plt.text(0.02, 0.98, stats_text, transform=plt.gca().transAxes,
             verticalalignment='top', fontsize=10,
             bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.8))

    # Add colorbar
    if nodes:
        cbar = plt.colorbar(nodes, shrink=0.8)
        cbar.set_label('Node Importance', rotation=270, labelpad=20)

    plt.axis('off')
    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    plt.close()  # Close instead of show for headless

    print(f"✅ Dependency graph saved to {output_file}")
    return output_file

def main():
    """Main visualization function"""
    print("🔍 CodeHUD Dependency Graph Visualizer")
    print("=" * 50)

    # Look for analysis data files
    possible_files = [
        "complete_viz/dependencies_visualization.json",
        "complete_viz/topology_visualization.json",
        "dependencies_visualization.json",
        "topology_visualization.json"
    ]

    analysis_data = {}
    data_file = None

    for file_path in possible_files:
        if Path(file_path).exists():
            print(f"📁 Loading analysis data from {file_path}")
            analysis_data = load_analysis_data(file_path)
            data_file = file_path
            break

    if not analysis_data:
        print("⚠️ No analysis data found. Run CodeHUD export first:")
        print("   cargo run -- export-viz .")
        print("\n🔄 Creating demo visualization with sample data...")
        analysis_data = {"content": {}}  # Empty data for demo
    else:
        print(f"✅ Loaded analysis data from {data_file}")

    # Extract dependencies
    print("\n🔗 Extracting dependency relationships...")
    dependencies = extract_dependencies(analysis_data)
    print(f"   Found {len(dependencies)} dependency relationships")

    # Create graph
    print("\n📊 Building dependency graph...")
    G = create_dependency_graph(dependencies, analysis_data)
    print(f"   Graph: {G.number_of_nodes()} nodes, {G.number_of_edges()} edges")

    # Create visualization
    print("\n🎨 Creating visualization...")
    output_file = visualize_dependency_graph(G)

    print("\n" + "=" * 50)
    print("🎯 Visualization Complete!")
    print(f"📊 Generated: {output_file}")
    print("🔍 This shows actual graphical charts vs text-based visualizations")

if __name__ == "__main__":
    # Check dependencies
    try:
        import matplotlib
        import networkx
        import numpy
    except ImportError as e:
        print(f"❌ Missing dependency: {e}")
        print("Install with: pip install matplotlib networkx numpy")
        sys.exit(1)

    main()