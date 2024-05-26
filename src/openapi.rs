#[cfg(not(test))]
use std::sync::Arc;

use aide::{
	openapi::{ApiKeyLocation, SecurityScheme, Tag},
	transform::TransformOpenApi,
};

#[cfg(not(test))]
use aide::{
	axum::{routing::get, ApiRouter},
	openapi::OpenApi,
};

#[cfg(not(test))]
use axum::{response::IntoResponse, Extension};

use crate::{error, extract::Json, session};

pub mod tag {
	pub const AUTH: &str = "Auth";
	pub const POST: &str = "Post";
	pub const KEY: &str = "Key";
}

#[cfg(not(test))]
pub fn routes() -> ApiRouter {
	use axum::response::Html;

	ApiRouter::new()
		.api_route(
			"/",
			get(|| async { Html(include_str!(concat!(env!("OUT_DIR"), "/scalar.html"))) }),
		)
		.route(
			"/private/api.json",
			get(|Extension(api): Extension<Arc<OpenApi>>| async move { Json(api).into_response() }),
		)
}

pub const SECURITY_SCHEME_API_KEY: &str = "APIKey";
pub const SECURITY_SCHEME_SESSION: &str = "Session";

pub fn docs(api: TransformOpenApi) -> TransformOpenApi {
	api.title("Axum Example Open API")
		.summary("An example Axum application")
		.description(include_str!("../docs/README.md"))
		.tag(Tag {
			name: tag::AUTH.into(),
			description: Some("User authentication".into()),
			..Default::default()
		})
		.tag(Tag {
			name: tag::POST.into(),
			description: Some("Post management".into()),
			..Default::default()
		})
		.tag(Tag {
			name: tag::KEY.into(),
			description: Some("API key management".into()),
			..Default::default()
		})
		.security_scheme(
			SECURITY_SCHEME_API_KEY,
			SecurityScheme::Http {
				scheme: "bearer".into(),
				bearer_format: Some("UUID".into()),
				description: Some("An API key".into()),
				extensions: Default::default(),
			},
		)
		.security_scheme(
			SECURITY_SCHEME_SESSION,
			SecurityScheme::ApiKey {
				location: ApiKeyLocation::Cookie,
				name: session::COOKIE_NAME.into(),
				description: Some("A session cookie".into()),
				extensions: Default::default(),
			},
		)
		.default_response_with::<Json<Vec<error::Message>>, _>(|res| {
			res.example(
				error::Message::new("error_code")
					.content("An optional human-readable error message.")
					.detail("key", "value")
					.into_vec(),
			)
		})
}
