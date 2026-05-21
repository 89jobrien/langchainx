"""Query engine: combines graph traversal with document provenance."""

from . import documents_store, graph_store


def query(query_text: str) -> dict:
    """Graph-only retrieval. Returns matching subgraph."""
    nodes = graph_store.search_nodes(query_text)
    if not nodes:
        return {"query": query_text, "subgraph": {"nodes": {}, "edges": []}}

    # BFS from best-matching node
    best = nodes[0]
    subgraph = graph_store.get_neighbors(best["name"], depth=2)
    return {"query": query_text, "subgraph": subgraph}


def query_with_evidence(query_text: str) -> dict:
    """Graph traversal + source chunk provenance."""
    result = query(query_text)
    subgraph = result["subgraph"]

    # Collect all doc_ids referenced in the subgraph
    doc_ids = set()
    for node in subgraph.get("nodes", {}).values():
        for src in node.get("sources", []):
            if src.get("doc_id"):
                doc_ids.add(src["doc_id"])
    for edge in subgraph.get("edges", []):
        for prov in edge.get("provenance", []):
            if prov.get("doc_id"):
                doc_ids.add(prov["doc_id"])

    # Fetch supporting chunks
    supporting = []
    for did in doc_ids:
        doc = documents_store.get_document(did)
        chunks = documents_store.get_chunks_for_document(did)
        if doc:
            supporting.append({
                "doc_id": did,
                "title": doc.get("title", ""),
                "source": doc.get("source", ""),
                "chunks": [c["text"] for c in chunks],
            })

    # Build evidence chain
    evidence = []
    for edge in subgraph.get("edges", []):
        for prov in edge.get("provenance", []):
            evidence.append({
                "claim": f"{edge['source']} --[{edge['relation']}]--> {edge['target']}",
                "confidence": edge["confidence"],
                "supporting_text": prov.get("supporting_text", ""),
                "doc_id": prov.get("doc_id", ""),
            })

    return {
        "query": query_text,
        "subgraph": subgraph,
        "supporting_documents": supporting,
        "evidence_chain": evidence,
    }
