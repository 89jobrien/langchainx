"""Raw document and chunk storage with provenance tracking.

All data persisted to data/documents.json. Documents are immutable once
ingested; chunks reference their parent doc_id.
"""

import hashlib
import json
import os
import re
from datetime import datetime, timezone
from pathlib import Path

DATA_DIR = Path(__file__).resolve().parent.parent / "data"
DOCS_FILE = DATA_DIR / "documents.json"
CHUNK_SIZE = 500  # characters per chunk


def _load():
    with open(DOCS_FILE) as f:
        return json.load(f)


def _save(data):
    with open(DOCS_FILE, "w") as f:
        json.dump(data, f, indent=2)


def _chunk_text(text: str, size: int = CHUNK_SIZE) -> list[str]:
    """Split text into chunks at sentence boundaries where possible."""
    sentences = re.split(r"(?<=[.!?])\s+", text)
    chunks = []
    current = ""
    for s in sentences:
        if len(current) + len(s) + 1 > size and current:
            chunks.append(current.strip())
            current = s
        else:
            current = f"{current} {s}" if current else s
    if current.strip():
        chunks.append(current.strip())
    return chunks if chunks else [text]


def ingest_document(
    doc_id: str,
    title: str,
    source: str,
    raw_content: str,
    metadata: dict | None = None,
) -> dict:
    """Store a document and its chunks. Returns chunk IDs."""
    data = _load()
    now = datetime.now(timezone.utc).isoformat()

    data["documents"][doc_id] = {
        "title": title,
        "source": source,
        "ingested_at": now,
        "metadata": metadata or {},
        "content_hash": hashlib.sha256(raw_content.encode()).hexdigest()[:16],
    }

    chunks = _chunk_text(raw_content)
    chunk_ids = []
    for i, chunk_text in enumerate(chunks):
        cid = f"{doc_id}_chunk_{i}"
        data["chunks"][cid] = {
            "doc_id": doc_id,
            "index": i,
            "text": chunk_text,
        }
        chunk_ids.append(cid)

    _save(data)
    return {"doc_id": doc_id, "chunk_count": len(chunks), "chunk_ids": chunk_ids}


def list_documents() -> list[dict]:
    """List all ingested documents (without chunk content)."""
    data = _load()
    return [
        {"doc_id": k, **{dk: dv for dk, dv in v.items() if dk != "content_hash"}}
        for k, v in data["documents"].items()
    ]


def get_document(doc_id: str) -> dict | None:
    data = _load()
    return data["documents"].get(doc_id)


def get_chunks_for_document(doc_id: str) -> list[dict]:
    data = _load()
    return [
        {"chunk_id": k, **v}
        for k, v in data["chunks"].items()
        if v["doc_id"] == doc_id
    ]


def search_chunks(query: str, max_results: int = 10) -> list[dict]:
    """Simple keyword search across all chunks."""
    data = _load()
    query_lower = query.lower()
    terms = query_lower.split()
    results = []
    for cid, chunk in data["chunks"].items():
        text_lower = chunk["text"].lower()
        score = sum(1 for t in terms if t in text_lower)
        if score > 0:
            results.append({"chunk_id": cid, "score": score, **chunk})
    results.sort(key=lambda x: x["score"], reverse=True)
    return results[:max_results]
