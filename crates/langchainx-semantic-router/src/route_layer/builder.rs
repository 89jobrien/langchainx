//! Builder for configuring and initializing a semantic route layer.
use std::sync::Arc;

use futures_util::future::try_join_all;

use langchainx_chain::language_models::llm::LLM;
use langchainx_chain::{LLMChain, LLMChainBuilder};
use langchainx_embedding::{Embedder, embedding::openai::OpenAiEmbedder};
use langchainx_llm::openai::OpenAI;
use langchainx_prompt::{HumanMessagePromptTemplate, template_jinja2};

use crate::{Index, MemoryIndex, RouteLayerBuilderError, Router};

use super::{AggregationMethod, RouteLayer};

/// Configures a [`RouteLayer`] and embeds routes that lack precomputed vectors.
/// ```rust,ignore
/// let capital_route = Router::new(
///     "capital",
///     &[
///         "Capital of France is Paris.",
///         "What is the capital of France?",
///     ],
/// );
/// let weather_route = Router::new(
///     "temperature",
///     &[
///         "What is the temperature?",
///         "Is it raining?",
///         "Is it cloudy?",
///     ],
/// );
/// let router_layer = RouteLayerBuilder::default()
///     .embedder(OpenAiEmbedder::default())
///     .add_route(capital_route)
///     .add_route(weather_route)
///     .aggregation_method(AggregationMethod::Sum)
///     .threshold(0.82)
///     .build()
///     .await
///     .unwrap();
/// ```
pub struct RouteLayerBuilder {
    embedder: Option<Arc<dyn Embedder>>,
    routes: Vec<Router>,
    threshold: Option<f64>,
    index: Option<Box<dyn Index>>,
    llm: Option<LLMChain>,
    top_k: usize,
    aggregation_method: AggregationMethod,
}
impl Default for RouteLayerBuilder {
    fn default() -> Self {
        Self::new()
            .embedder(OpenAiEmbedder::default())
            .llm(OpenAI::default())
            .index(MemoryIndex::new())
    }
}

impl RouteLayerBuilder {
    /// Creates an unconfigured builder with `top_k` 5 and sum aggregation.
    pub fn new() -> Self {
        Self {
            embedder: None,
            routes: Vec::new(),
            threshold: None,
            llm: None,
            index: None,
            top_k: 5,
            aggregation_method: AggregationMethod::Sum,
        }
    }

    /// Sets the number of utterance matches considered, coercing zero to one.
    pub fn top_k(mut self, top_k: usize) -> Self {
        let mut top_k = top_k;
        if top_k == 0 {
            log::warn!("top_k cannot be 0, setting it to 1");
            top_k = 1;
        }
        self.top_k = top_k;
        self
    }

    /// Sets the language model used to generate tool input for matched routes.
    pub fn llm<L: LLM + 'static>(mut self, llm: L) -> Self {
        let prompt = HumanMessagePromptTemplate::new(template_jinja2!(
            "You should Generate the input for the following tool.
Tool description:{{description}}.
Input query context to generate the input for the tool :{{query}}

Tool Input:
",
            "description",
            "query"
        ));
        // SAFETY(#88): prompt and LLM are unconditionally set above; build is infallible here.
        let chain = LLMChainBuilder::new()
            .prompt(prompt)
            .llm(llm)
            .build()
            .expect("RouteLayerBuilder::llm: prompt and LLM are always set");
        self.llm = Some(chain);
        self
    }

    /// Sets the route index implementation.
    pub fn index<I: Index + 'static>(mut self, index: I) -> Self {
        self.index = Some(Box::new(index));
        self
    }

    /// Sets the embedder used for route utterances and incoming queries.
    pub fn embedder<E: Embedder + 'static>(mut self, embedder: E) -> Self {
        self.embedder = Some(Arc::new(embedder));
        self
    }

    /// Sets the minimum similarity score for a candidate utterance match.
    pub fn threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self
    }

    /// Adds a route to initialize during [`build`](Self::build).
    pub fn add_route(mut self, route: Router) -> Self {
        self.routes.push(route);
        self
    }

    /// Sets how multiple utterance scores for one route are combined.
    pub fn aggregation_method(mut self, aggregation_method: AggregationMethod) -> Self {
        self.aggregation_method = aggregation_method;
        self
    }

    /// Embeds routes as needed, inserts them into the index, and builds the layer.
    pub async fn build(mut self) -> Result<RouteLayer, RouteLayerBuilderError> {
        const DEFAULT_THRESHOLD: f64 = 0.82;
        let embedder = self
            .embedder
            .ok_or(RouteLayerBuilderError::MissingEmbedder)?;
        let index = self.index.ok_or(RouteLayerBuilderError::MissingIndex)?;
        let llm = self.llm.ok_or(RouteLayerBuilderError::MissingLLM)?;
        let mut router = RouteLayer {
            embedder,
            index,
            llm,
            threshold: self.threshold.unwrap_or(DEFAULT_THRESHOLD),
            top_k: self.top_k,
            aggregation_method: self.aggregation_method,
        };

        let embedding_futures = self
            .routes
            .iter_mut()
            .filter_map(|route| {
                if route.embedding.is_none() {
                    Some(router.embedder.embed_documents(&route.utterances))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let embeddings = try_join_all(embedding_futures).await?;

        for (route, embedding) in self
            .routes
            .iter_mut()
            .filter(|r| r.embedding.is_none())
            .zip(embeddings)
        {
            route.embedding = Some(embedding);
        }

        router.index.add(&self.routes).await?;

        Ok(router)
    }
}
