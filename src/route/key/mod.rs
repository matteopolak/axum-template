use std::borrow::Cow;

use aide::axum::{routing::get_with, ApiRouter};
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{error, AppState};

pub mod model;
pub mod route;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("unknown key {0}")]
	UnknownKey(Uuid),
}

type RouteError = error::RouteError<Error>;

pub fn routes() -> ApiRouter<AppState> {
	use route::*;

	ApiRouter::new()
		.api_route(
			"/",
			get_with(list_keys, list_keys_docs).post_with(create_key, create_key_docs),
		)
		.api_route(
			"/:id",
			get_with(get_key, get_key_docs).delete_with(delete_key, delete_key_docs),
		)
}

impl error::ErrorShape for Error {
	fn status(&self) -> StatusCode {
		match self {
			Self::UnknownKey(..) => StatusCode::NOT_FOUND,
		}
	}

	fn errors(&self) -> Vec<error::Message<'_>> {
		match self {
			Self::UnknownKey(key) => vec![error::Message {
				content: "unknown_key".into(),
				field: None,
				details: Some(Cow::Owned({
					let mut map = error::Map::new();
					map.insert("key".into(), json!(key));
					map
				})),
			}],
		}
	}
}
