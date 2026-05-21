"""ContextGraphSkill: main entry point tying all layers together.

Usage:
    from scripts.contextgraph import ContextGraphSkill
    from scripts.tools import wiki_store

    skill = ContextGraphSkill()
    result = skill.ingest_with_content(...)
    answer = skill.query_with_evidence(...)
"""

from . import documents_store, graph_store, retrieval_engine


class ContextGraphSkill:
    """Facade over raw-source, graph, and retrieval layers."""

    def ingest_with_content(
        self,
        doc_id: str,
        title: str,
        source: str,
        raw_content: str,
        entities: list[dict],
        relations: list[dict],
        metadata: dict | None = None,
    ) -> dict:
        """Full ingest: store document, add graph nodes/edges with provenance."""
        doc_result = documents_store.ingest_document(
            doc_id=doc_id,
            title=title,
            source=source,
            raw_content=raw_content,
            metadata=metadata,
        )

        nodes_added = 0
        for ent in entities:
            graph_store.add_node(
                name=ent["name"],
                node_type=ent["type"],
                supporting_text=ent.get("supporting_text", ""),
                doc_id=doc_id,
            )
            nodes_added += 1

        edges_added = 0
        for rel in relations:
            edge = graph_store.add_edge(
                source_name=rel["source"],
                target_name=rel["target"],
                relation=rel["type"],
                confidence=rel.get("confidence", 0.8),
                supporting_text=rel.get("supporting_text", ""),
                doc_id=doc_id,
            )
            if edge:
                edges_added += 1

        return {
            "doc_id": doc_id,
            "chunk_count": doc_result["chunk_count"],
            "nodes_added": nodes_added,
            "edges_added": edges_added,
        }

    def add_node(self, name: str, node_type: str) -> str:
        return graph_store.add_node(name, node_type)

    def add_edge(
        self, source_name: str, target_name: str, relation: str, confidence: float
    ) -> dict | None:
        return graph_store.add_edge(source_name, target_name, relation, confidence)

    def query(self, query_text: str) -> dict:
        return retrieval_engine.query(query_text)

    def query_with_evidence(self, query_text: str) -> dict:
        return retrieval_engine.query_with_evidence(query_text)
