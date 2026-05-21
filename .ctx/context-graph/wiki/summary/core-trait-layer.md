---
title: Core Trait Layer
source_document: core_traits
tags: [summary, traits, architecture]
---

# Core Trait Layer

Primary async trait abstractions defining the langchainx API.

## Traits

- [[llm]] -- generate, invoke, stream over messages
- [[chain]] -- call, invoke, execute, stream over prompt args
- [[tool]] -- name, description, parameters (JSON schema), run
- [[embedder]] -- embed_documents, embed_query
- [[vectorstore]] -- add_documents, similarity_search
- [[retriever]] -- get_relevant_documents
- [[basememory]] -- messages, add_message, clear
- [[loader]] -- load -> Stream<Document>
- [[agent]] -- plan, get_tools
- [[formatprompter]] -- format_prompt -> PromptValue
- [[outputparser]] -- parse string output

## Key Types

- [[generateresult]] -- generation string + optional [[tokenusage]]
- [[message]] -- content + [[messagetype]] + optional tool_calls/images
- [[document]] -- page_content + metadata HashMap + score
- [[promptargs]] -- HashMap<String, Value>
- [[promptvalue]] -- formatted prompt with to_chat_messages()
- [[prompttemplate]] -- template string with FString or Jinja2 format
