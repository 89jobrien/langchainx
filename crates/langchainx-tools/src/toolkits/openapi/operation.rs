use serde_json::Value;
use url::Url;

use super::OperationContext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

#[derive(Clone, Debug)]
pub(crate) struct OpenApiParameter {
    pub(crate) name: String,
    pub(crate) location: ParameterLocation,
    pub(crate) required: bool,
    pub(crate) explode: bool,
    pub(crate) schema: Value,
}

#[derive(Clone, Debug)]
pub(crate) struct OpenApiOperation {
    pub(crate) exposed_name: String,
    pub(crate) operation_id: String,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) server: Url,
    pub(crate) parameters: Vec<OpenApiParameter>,
    pub(crate) body_schema: Option<Value>,
    pub(crate) body_required: bool,
    pub(crate) tool_schema: Value,
}

impl OpenApiOperation {
    pub(crate) fn context(&self) -> OperationContext {
        OperationContext::new(&self.method, &self.path, &self.operation_id)
    }

    pub(crate) fn description(&self) -> String {
        let mut path = self.path.chars().take(200).collect::<String>();
        if path.chars().count() < self.path.chars().count() {
            path.push_str("...");
        }
        format!("Call the {} {} operation.", self.method, path)
    }
}
