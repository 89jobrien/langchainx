//! Semantic route selection and optional tool-input generation.
use std::{collections::HashMap, sync::Arc};

use serde_json::Value;

use langchainx_chain::{Chain, LLMChain, prompt_args};
use langchainx_embedding::Embedder;

use crate::{Index, RouteLayerError, Router};

/// Strategy for combining multiple utterance scores for one route.
pub enum AggregationMethod {
    /// Arithmetic mean of the scores.
    Mean,
    /// Highest score.
    Max,
    /// Sum of the scores.
    Sum,
}
impl AggregationMethod {
    /// Aggregates scores, returning `0.0` for an empty slice.
    pub fn aggregate(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        match self {
            AggregationMethod::Sum => values.iter().sum(),
            AggregationMethod::Mean => values.iter().sum::<f64>() / values.len() as f64,
            AggregationMethod::Max => *values
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(&0.0),
        }
    }
}

#[derive(Debug, Clone)]
/// The route selected for an input query.
pub struct RouteChoise {
    /// Selected route name.
    pub route: String,
    /// Highest individual utterance similarity for the selected route.
    pub similarity_score: f64,
    /// LLM-generated tool input when the route has a tool description.
    pub tool_input: Option<Value>,
}

/// Embeds queries, searches an index, and selects the highest-scoring route.
pub struct RouteLayer {
    pub(crate) embedder: Arc<dyn Embedder>,
    pub(crate) index: Box<dyn Index>,
    pub(crate) threshold: f64,
    pub(crate) llm: LLMChain,
    pub(crate) top_k: usize,
    pub(crate) aggregation_method: AggregationMethod,
}

impl RouteLayer {
    /// Embeds routes that need vectors and adds all routes to the index.
    pub async fn add_routes(&mut self, routers: &mut [Router]) -> Result<(), RouteLayerError> {
        for router in routers.iter_mut() {
            if router.embedding.is_none() {
                let embeddigns = self.embedder.embed_documents(&router.utterances).await?;
                router.embedding = Some(embeddigns);
            }
        }
        self.index.add(routers).await?;
        Ok(())
    }

    /// Deletes a route from the index by name.
    pub async fn delete_route<S: Into<String>>(
        &mut self,
        route_name: S,
    ) -> Result<(), RouteLayerError> {
        self.index.delete(&route_name.into()).await?;
        Ok(())
    }

    /// Returns all routes currently stored in the index.
    pub async fn get_routers(&self) -> Result<Vec<Router>, RouteLayerError> {
        let routes = self.index.get_routers().await?;
        Ok(routes)
    }

    async fn filter_similar_routes(
        &self,
        query_vector: &[f64],
    ) -> Result<Vec<(String, f64)>, RouteLayerError> {
        let similar_routes = self.index.query(query_vector, self.top_k).await?;

        Ok(similar_routes
            .into_iter()
            .filter(|(_, score)| *score >= self.threshold)
            .collect())
    }

    fn compute_total_scores(&self, similar_routes: &[(String, f64)]) -> HashMap<String, f64> {
        let mut scores_by_route: HashMap<String, Vec<f64>> = HashMap::new();

        for (route_name, score) in similar_routes {
            scores_by_route
                .entry(route_name.to_owned())
                .or_default()
                .push(*score);
        }

        scores_by_route
            .into_iter()
            .map(|(route, scores)| {
                let aggregated_score = self.aggregation_method.aggregate(&scores);
                (route, aggregated_score)
            })
            .collect()
    }

    fn find_top_route_and_scores(
        &self,
        total_scores: HashMap<String, f64>,
        scores_by_route: &HashMap<String, Vec<f64>>,
    ) -> (Option<String>, Vec<f64>) {
        let top_route = total_scores
            .into_iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(route, _)| route);

        let mut top_scores = top_route
            .as_ref()
            .and_then(|route| scores_by_route.get(route))
            .unwrap_or(&vec![])
            .clone();

        top_scores.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        (top_route, top_scores)
    }

    /// Selects the best route for a text query.
    ///
    /// Generates tool input when the selected route has a tool description.
    pub async fn call<S: Into<String>>(
        &self,
        query: S,
    ) -> Result<Option<RouteChoise>, RouteLayerError> {
        let query: String = query.into();
        let query_vector = self.embedder.embed_query(&query).await?;

        let route_choise = self.call_embedding(&query_vector).await?;

        let Some(choice) = route_choise else {
            return Ok(None);
        };

        let router = self.index.get_router(&choice.route).await?;

        let Some(description) = router.tool_description else {
            return Ok(Some(choice));
        };

        let tool_input = self.generate_tool_input(&query, &description).await?;

        Ok(Some(RouteChoise {
            tool_input: Some(tool_input),
            ..choice
        }))
    }

    /// Selects the best route for a precomputed embedding without generating tool input.
    pub async fn call_embedding(
        &self,
        embedding: &[f64],
    ) -> Result<Option<RouteChoise>, RouteLayerError> {
        let similar_routes = self.filter_similar_routes(embedding).await?;

        if similar_routes.is_empty() {
            return Ok(None);
        }

        // Correctly collect scores by route manually
        let mut scores_by_route: HashMap<String, Vec<f64>> = HashMap::new();
        for (route_name, score) in &similar_routes {
            scores_by_route
                .entry(route_name.clone())
                .or_default()
                .push(*score);
        }

        let total_scores = self.compute_total_scores(&similar_routes);

        let (top_route, top_scores) =
            self.find_top_route_and_scores(total_scores, &scores_by_route);

        Ok(top_route.map(|route| RouteChoise {
            route,
            similarity_score: top_scores[0],
            tool_input: None,
        }))
    }

    async fn generate_tool_input(
        &self,
        query: &str,
        description: &str,
    ) -> Result<Value, RouteLayerError> {
        let output = self
            .llm
            .invoke(prompt_args! {
                "description"=>description,
                "query"=>query
            })
            .await?;
        match serde_json::from_str::<Value>(&output) {
            Ok(value_result) => Ok(value_result),
            Err(_) => Ok(Value::String(output)),
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use langchainx_chain::test_utils::FakeLLM;
    use langchainx_embedding::EmbedderError;

    use crate::{MemoryIndex, RouteLayerBuilder};

    use super::*;

    struct FakeEmbedder {
        query_vec: Vec<f64>,
    }

    impl FakeEmbedder {
        fn new(query_vec: Vec<f64>) -> Self {
            Self { query_vec }
        }
    }

    #[async_trait]
    impl langchainx_embedding::Embedder for FakeEmbedder {
        async fn embed_documents(
            &self,
            documents: &[String],
        ) -> Result<Vec<Vec<f64>>, EmbedderError> {
            Ok(documents.iter().map(|_| self.query_vec.clone()).collect())
        }

        async fn embed_query(&self, _text: &str) -> Result<Vec<f64>, EmbedderError> {
            Ok(self.query_vec.clone())
        }
    }

    async fn build_test_layer(query_vec: Vec<f64>) -> RouteLayer {
        let greet = Router::new("greet", &["hello", "hi"])
            .with_embedding(vec![vec![1.0, 0.0, 0.0], vec![1.0, 0.0, 0.0]]);
        let weather = Router::new("weather", &["rain", "sun"])
            .with_embedding(vec![vec![0.0, 0.0, 1.0], vec![0.0, 0.0, 1.0]]);

        RouteLayerBuilder::new()
            .embedder(FakeEmbedder::new(query_vec))
            .llm(FakeLLM::new(vec![]))
            .index(MemoryIndex::new())
            .threshold(0.5)
            .add_route(greet)
            .add_route(weather)
            .build()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_call_embedding_routes_to_greet() {
        let layer = build_test_layer(vec![1.0, 0.0, 0.0]).await;
        let result = layer.call_embedding(&[1.0, 0.0, 0.0]).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().route, "greet");
    }

    #[tokio::test]
    async fn test_call_embedding_routes_to_weather() {
        let layer = build_test_layer(vec![0.0, 0.0, 1.0]).await;
        let result = layer.call_embedding(&[0.0, 0.0, 1.0]).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().route, "weather");
    }

    #[tokio::test]
    async fn test_call_embedding_below_threshold_returns_none() {
        let layer = build_test_layer(vec![0.0, 1.0, 0.0]).await;
        let result = layer.call_embedding(&[0.0, 1.0, 0.0]).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_route_layer_similarity_score_is_reasonable() {
        let layer = build_test_layer(vec![1.0, 0.0, 0.0]).await;
        let result = layer
            .call_embedding(&[1.0, 0.0, 0.0])
            .await
            .unwrap()
            .unwrap();
        assert!((result.similarity_score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_mean_aggregate_empty_slice() {
        // Should return 0.0, not panic with division by zero
        assert_eq!(AggregationMethod::Mean.aggregate(&[]), 0.0);
    }

    #[test]
    fn test_max_aggregate_empty_slice() {
        assert_eq!(AggregationMethod::Max.aggregate(&[]), 0.0);
    }

    #[test]
    fn test_sum_aggregate_empty_slice() {
        assert_eq!(AggregationMethod::Sum.aggregate(&[]), 0.0);
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        fn finite_f64() -> impl Strategy<Value = f64> {
            (-1e10f64..1e10f64)
        }

        proptest! {
            #[test]
            fn mean_between_min_and_max(values in prop::collection::vec(finite_f64(), 1..100)) {
                let mean = AggregationMethod::Mean.aggregate(&values);
                let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                prop_assert!(mean >= min && mean <= max,
                    "mean {} not in [{}, {}]", mean, min, max);
            }

            #[test]
            fn sum_gte_max_for_non_negative(values in prop::collection::vec(0.0f64..1e10, 1..100)) {
                let sum = AggregationMethod::Sum.aggregate(&values);
                let max = AggregationMethod::Max.aggregate(&values);
                prop_assert!(sum >= max,
                    "sum {} < max {}", sum, max);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_route_layer_builder() {
        use langchainx_embedding::embedding::openai::OpenAiEmbedder;
        use langchainx_llm::openai::OpenAI;

        let captial_route = Router::new(
            "captial",
            &[
                "Capital of France is Paris.",
                "What is the captial of France?",
            ],
        );
        let description = String::from(
            r#""A wrapper around Google Search. "
	"Useful for when you need to answer questions about current events. "
	"Always one of the first options when you need to find information on internet"
	"Input should be a search query."#,
        );

        let weather_route = Router::new(
            "temperature",
            &[
                "What is the temperature?",
                "Is it raining?",
                "Is it cloudy?",
            ],
        )
        .with_tool_description(description);
        let router_layer = RouteLayerBuilder::default()
            .embedder(OpenAiEmbedder::default())
            .add_route(captial_route)
            .add_route(weather_route)
            .aggregation_method(AggregationMethod::Sum)
            .build()
            .await
            .unwrap();
        let routes = router_layer
            .call("What is the temperature in Peru?")
            .await
            .unwrap();

        println!("{:?}", routes);
        assert_eq!(routes.unwrap().route, "temperature");
    }
}
