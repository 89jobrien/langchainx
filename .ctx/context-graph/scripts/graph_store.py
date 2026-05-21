"""Entity-relation graph store backed by data/graph.json.

Nodes are deduplicated by normalized name. Edges carry confidence and
provenance (source doc_id + supporting_text).
"""

import json
import re
from collections import deque
from pathlib import Path

DATA_DIR = Path(__file__).resolve().parent.parent / "data"
GRAPH_FILE = DATA_DIR / "graph.json"

MIN_CONFIDENCE = 0.6
MAX_GRAPH_DEPTH = 2
MAX_NODES = 50


def _load():
    with open(GRAPH_FILE) as f:
        return json.load(f)


def _save(data):
    with open(GRAPH_FILE, "w") as f:
        json.dump(data, f, indent=2)


def _normalize(name: str) -> str:
    """Normalize entity name to a slug."""
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")


def add_node(
    name: str,
    node_type: str,
    supporting_text: str = "",
    doc_id: str = "",
) -> str:
    """Add or update a node. Returns the node slug."""
    data = _load()
    slug = _normalize(name)
    existing = data["nodes"].get(slug, {})
    sources = existing.get("sources", [])
    if doc_id and doc_id not in [s.get("doc_id") for s in sources]:
        sources.append({"doc_id": doc_id, "supporting_text": supporting_text})

    data["nodes"][slug] = {
        "name": name,
        "type": node_type,
        "slug": slug,
        "sources": sources,
    }
    _save(data)
    return slug


def add_edge(
    source_name: str,
    target_name: str,
    relation: str,
    confidence: float,
    supporting_text: str = "",
    doc_id: str = "",
) -> dict | None:
    """Add an edge if confidence >= MIN_CONFIDENCE. Returns edge or None."""
    if confidence < MIN_CONFIDENCE:
        return None

    data = _load()
    src = _normalize(source_name)
    tgt = _normalize(target_name)

    # Deduplicate: same src/tgt/relation
    for edge in data["edges"]:
        if edge["source"] == src and edge["target"] == tgt and edge["relation"] == relation:
            # Update confidence if higher, append provenance
            if confidence > edge["confidence"]:
                edge["confidence"] = confidence
            if doc_id:
                prov = edge.get("provenance", [])
                if doc_id not in [p.get("doc_id") for p in prov]:
                    prov.append({"doc_id": doc_id, "supporting_text": supporting_text})
                    edge["provenance"] = prov
            _save(data)
            return edge

    edge = {
        "source": src,
        "target": tgt,
        "relation": relation,
        "confidence": confidence,
        "provenance": [{"doc_id": doc_id, "supporting_text": supporting_text}] if doc_id else [],
    }
    data["edges"].append(edge)
    _save(data)
    return edge


def get_node(name: str) -> dict | None:
    data = _load()
    return data["nodes"].get(_normalize(name))


def get_neighbors(name: str, depth: int = 1) -> dict:
    """BFS traversal from a node up to `depth` hops. Returns subgraph."""
    if depth > MAX_GRAPH_DEPTH:
        depth = MAX_GRAPH_DEPTH

    data = _load()
    slug = _normalize(name)
    if slug not in data["nodes"]:
        return {"nodes": {}, "edges": []}

    visited = set()
    queue = deque([(slug, 0)])
    result_nodes = {}
    result_edges = []

    while queue and len(result_nodes) < MAX_NODES:
        current, d = queue.popleft()
        if current in visited:
            continue
        visited.add(current)
        if current in data["nodes"]:
            result_nodes[current] = data["nodes"][current]

        if d < depth:
            for edge in data["edges"]:
                if edge["confidence"] < MIN_CONFIDENCE:
                    continue
                neighbor = None
                if edge["source"] == current:
                    neighbor = edge["target"]
                elif edge["target"] == current:
                    neighbor = edge["source"]
                if neighbor and neighbor not in visited:
                    result_edges.append(edge)
                    queue.append((neighbor, d + 1))

    return {"nodes": result_nodes, "edges": result_edges}


def list_nodes() -> list[dict]:
    data = _load()
    return list(data["nodes"].values())


def list_edges() -> list[dict]:
    data = _load()
    return data["edges"]


def search_nodes(query: str) -> list[dict]:
    """Search nodes by name substring."""
    data = _load()
    q = query.lower()
    return [n for n in data["nodes"].values() if q in n["name"].lower()]
