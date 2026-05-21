# Ingestion Rules

## Entity Extraction

1. Only extract entities explicitly mentioned in the source text.
2. Every entity must have a `supporting_text` snippet from the source.
3. Normalize entity types using the ontology (see ontology.md).
4. Do not infer entities that are implied but not stated.

## Relation Extraction

1. Only add relations with direct textual evidence.
2. Provide `supporting_text` for every relation.
3. Do not add edges with confidence below 0.6.
4. Confidence scale:
   - 1.0: explicitly stated ("X causes Y")
   - 0.8: strongly implied ("X leads to Y")
   - 0.6: weakly implied ("X may relate to Y")
   - Below 0.6: do not add

## Post-Ingest Steps

1. Write a wiki summary page for the document.
2. Update or create entity pages for each extracted entity.
3. Update topic pages if the document touches existing themes.
4. Flag contradictions when new data conflicts with existing claims.
