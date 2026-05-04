use std::{collections::HashMap, sync::Arc};

use serde_json::Value;

use crate::{
    chain::{Chain, LLMChain},
    embedding::Embedder,
    prompt_args,
    semantic_router::{Index, RouteLayerError, Router},
};

pub enum AggregationMethod {
    Mean,
    Max,
    Sum,
}
impl AggregationMethod {
    pub fn aggregate(&self, values: &[f64]) -> f64 {
        match self {
            AggregationMethod::Sum => values.iter().sum(),
            AggregationMethod::Mean => values.iter().sum::<f64>() / values.len() as f64,
            AggregationMethod::Max => *values
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouteChoise {
    pub route: String,
    pub similarity_score: f64,
    pub tool_input: Option<Value>,
}

pub struct RouteLayer {
    pub(crate) embedder: Arc<dyn Embedder>,
    pub(crate) index: Box<dyn Index>,
    pub(crate) threshold: f64,
    pub(crate) llm: LLMChain,
    pub(crate) top_k: usize,
    pub(crate) aggregation_method: AggregationMethod,
}

impl RouteLayer {
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

    pub async fn delete_route<S: Into<String>>(
        &mut self,
        route_name: S,
    ) -> Result<(), RouteLayerError> {
        self.index.delete(&route_name.into()).await?;
        Ok(())
    }

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

    /// Call the route layer with a query and return the best route choise
    /// If route has a tool description, it will also return the tool input
    pub async fn call<S: Into<String>>(
        &self,
        query: S,
    ) -> Result<Option<RouteChoise>, RouteLayerError> {
        let query: String = query.into();
        let query_vector = self.embedder.embed_query(&query).await?;

        let route_choise = self.call_embedding(&query_vector).await?;

        if route_choise.is_none() {
            return Ok(None);
        }

        let router = self
            .index
            .get_router(&route_choise.as_ref().unwrap().route) //safe to unwrap
            .await?;

        if router.tool_description.is_none() {
            return Ok(route_choise);
        }

        let tool_input = self
            .generate_tool_input(&query, &router.tool_description.unwrap())
            .await?;

        Ok(route_choise.map(|route| RouteChoise {
            tool_input: Some(tool_input),
            ..route
        }))
    }

    /// Call the route layer with a query and return the best route choise
    /// If route has a tool description, it will not return the tool input,
    /// this just returns the route
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

    use crate::{
        embedding::{EmbedderError, openai::OpenAiEmbedder},
        semantic_router::{MemoryIndex, RouteLayerBuilder},
        test_utils::FakeLLM,
    };

    use super::*;

    // ---------------------------------------------------------------------------
    // Inline fake embedder — deterministic, offline, no API keys needed.
    // Encodes each text as a fixed unit-vector chosen by index in the call order.
    // ---------------------------------------------------------------------------
    struct FakeEmbedder {
        /// Each call to embed_query returns the next vector in this list (cycling).
        /// embed_documents returns one vector per document.
        query_vec: Vec<f64>,
    }

    impl FakeEmbedder {
        fn new(query_vec: Vec<f64>) -> Self {
            Self { query_vec }
        }
    }

    #[async_trait]
    impl crate::embedding::Embedder for FakeEmbedder {
        async fn embed_documents(
            &self,
            documents: &[String],
        ) -> Result<Vec<Vec<f64>>, EmbedderError> {
            // Return the same query_vec for every document
            Ok(documents.iter().map(|_| self.query_vec.clone()).collect())
        }

        async fn embed_query(&self, _text: &str) -> Result<Vec<f64>, EmbedderError> {
            Ok(self.query_vec.clone())
        }
    }

    // Build a RouteLayer with two routes (greet, weather) using a FakeEmbedder
    // that returns orthogonal unit vectors, so cosine similarity is deterministic.
    async fn build_test_layer(query_vec: Vec<f64>) -> RouteLayer {
        // greet utterances embed to [1,0,0]; weather utterances embed to [0,0,1]
        // We pre-assign embeddings so the embedder is only used for query embedding.
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
        // Query vector aligns with "greet"
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
        // [0,1,0] is orthogonal to both routes — cosine similarity == 0 < threshold 0.5
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
        // Exact match → similarity should be 1.0
        assert!((result.similarity_score - 1.0).abs() < 1e-9);
    }

    #[tokio::test]
    #[ignore]
    async fn test_route_layer_builder() {
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
