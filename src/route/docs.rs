use std::sync::Arc;

use aide::{
	axum::{
		routing::{get, get_with},
		ApiRouter, IntoApiResponse,
	},
	openapi::OpenApi,
	scalar::Scalar,
};
use axum::{response::IntoResponse, Extension};

use crate::extract::Json;

pub fn routes() -> ApiRouter {
	ApiRouter::new()
		.api_route_with(
			"/",
			get_with(
				Scalar::new("/docs/private/api.json")
					.with_title("Axum Template")
					.axum_handler(),
				|op| op.description("This documentation page."),
			),
			|p| p.security_requirement("Cookie"),
		)
		.route("/private/api.json", get(serve_docs))
}

async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
	Json(api).into_response()
}
