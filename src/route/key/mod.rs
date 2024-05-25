use aide::axum::{routing::get_with, ApiRouter};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::{error, AppState};

pub mod model;
pub mod route;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
	#[error("key_not_found")]
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

	fn into_errors(self) -> Vec<error::Message<'static>> {
		let message = match self {
			Self::UnknownKey(..) => "The key you provided does not exist.",
		};

		let message = error::Message::new(self.to_string()).content(message);
		let Self::UnknownKey(key) = self;

		message.detail("key", key.to_string()).into_vec()
	}
}
