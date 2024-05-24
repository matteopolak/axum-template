use aide::axum::{routing::get_with, ApiRouter};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::{error, AppState};

pub mod model;
pub mod route;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("post_not_found")]
	UnknownPost(Uuid),
}

pub type RouteError = error::RouteError<Error>;

pub fn routes() -> ApiRouter<AppState> {
	use route::*;

	ApiRouter::new()
		.api_route(
			"/",
			get_with(get_posts, get_posts_docs).post_with(create_post, create_post_docs),
		)
		.api_route("/me", get_with(get_user_posts, get_user_posts_docs))
		.api_route(
			"/:id",
			get_with(get_post, get_post_docs)
				.put_with(update_post, update_post_docs)
				.delete_with(delete_post, delete_post_docs),
		)
}

impl error::ErrorShape for Error {
	fn status(&self) -> StatusCode {
		match self {
			Self::UnknownPost(..) => StatusCode::NOT_FOUND,
		}
	}

	fn into_errors(self) -> Vec<error::Message<'static>> {
		let message = match self {
			Self::UnknownPost(..) => "The post you provided does not exist.",
		};

		let message = error::Message::new(self.to_string()).message(message);
		let Self::UnknownPost(key) = self;

		vec![message.detail("key", key.to_string())]
	}
}
