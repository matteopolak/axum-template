use std::borrow::Cow;

use aide::{
	openapi::{ApiKeyLocation, SecurityScheme, Tag},
	transform::TransformOpenApi,
};

use crate::{error, extract::Json, session};

pub mod tag {
	pub const AUTH: &str = "Auth";
	pub const POST: &str = "Post";
	pub const KEY: &str = "Key";
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
			SecurityScheme::ApiKey {
				location: ApiKeyLocation::Header,
				name: "X-API-Key".into(),
				description: Some("An API key".into()),
				extensions: Default::default(),
			},
		)
		.security_scheme(
			"Session",
			SecurityScheme::ApiKey {
				location: ApiKeyLocation::Cookie,
				name: session::COOKIE_NAME.into(),
				description: Some("A user session cookie".into()),
				extensions: Default::default(),
			},
		)
		.default_response_with::<Json<error::Message>, _>(|res| {
			res.example(error::Message {
				content: "error message".into(),
				field: Some("optional field".into()),
				details: Some(Cow::Owned({
					let mut map = error::Map::new();
					map.insert("key".into(), serde_json::json!("value"));
					map
				})),
			})
		})
}
