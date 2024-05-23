use std::borrow::Cow;
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
	scalar::Scalar,
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
	ApiRouter::new()
		.api_route(
			"/",
			get(Scalar::new("/docs/private/api.json")
				.with_title("Axum Template")
				.axum_handler()),
		)
		.route(
			"/private/api.json",
			get(|Extension(api): Extension<Arc<OpenApi>>| async move { Json(api).into_response() }),
		)
}

pub fn docs(api: TransformOpenApi) -> TransformOpenApi {
	api.title("Axum Example Open API")
		.summary("An example Axum application")
		.description(include_str!("../README.md"))
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
			"API Key",
			SecurityScheme::Http {
				scheme: "bearer".into(),
				bearer_format: Some("UUID".into()),
				description: Some("An API key".into()),
				extensions: Default::default(),
			},
		)
		.security_scheme(
			"Session",
			SecurityScheme::ApiKey {
				location: ApiKeyLocation::Cookie,
				name: session::COOKIE_NAME.into(),
				description: Some("A session cookie".into()),
				extensions: Default::default(),
			},
		)
		.default_response_with::<Json<Vec<error::Message>>, _>(|res| {
			res.example(vec![error::Message {
				content: "error message".into(),
				field: Some("optional field".into()),
				details: Some(Cow::Owned({
					let mut map = error::Map::new();
					map.insert("key".into(), serde_json::json!("value"));
					map
				})),
			}])
		})
}
