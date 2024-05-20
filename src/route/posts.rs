use std::borrow::Cow;

use aide::axum::{
	routing::{get_with, post_with},
	ApiRouter,
};
use axum::{
	extract::{Path, State},
	http::StatusCode,
};
use macros::route;
use serde_json::json;
use uuid::Uuid;

use crate::{
	error,
	extract::{Json, Query, Session},
	model,
	openapi::tag,
	AppState, Database,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("unknown post {0}")]
	UnknownPost(Uuid),
}

impl error::ErrorShape for Error {
	fn status(&self) -> StatusCode {
		match self {
			Self::UnknownPost(..) => StatusCode::NOT_FOUND,
		}
	}

	fn errors(&self) -> Vec<error::Message<'_>> {
		match self {
			Self::UnknownPost(post) => vec![error::Message {
				content: "unknown_post".into(),
				field: None,
				details: Some(Cow::Owned({
					let mut map = error::Map::new();
					map.insert("post".into(), json!(post));
					map
				})),
			}],
		}
	}
}

type RouteError = error::RouteError<Error>;

pub fn routes() -> ApiRouter<AppState> {
	ApiRouter::new()
		.api_route("/", get_with(get_all_posts, get_all_posts_docs))
		.api_route("/me", post_with(get_user_posts, get_user_posts_docs))
		.api_route("/:id", post_with(get_one_post, get_one_post_docs))
}

/// Get own posts
/// Returns a paginated response of your posts, newest first.
#[route(tag = tag::POST)]
async fn get_user_posts(
	State(database): State<Database>,
	session: Session,
	Query(paginate): Query<super::PaginateInput>,
) -> Result<Json<Vec<model::Post>>, RouteError> {
	let posts = sqlx::query_as!(
		model::Post,
		r#"
			SELECT * FROM post
			WHERE user_id = $1
			ORDER BY created_at DESC
			LIMIT $2 OFFSET $3
		"#,
		session.user.id,
		paginate.limit(),
		paginate.offset(),
	)
	.fetch_all(&database)
	.await?;

	Ok(Json(posts))
}

/// Get all posts
/// Returns a paginated response of all posts, newest first.
#[route(tag = tag::POST)]
async fn get_all_posts(
	State(database): State<Database>,
	Query(paginate): Query<super::PaginateInput>,
) -> Result<Json<Vec<model::Post>>, RouteError> {
	let posts = sqlx::query_as!(
		model::Post,
		r#"
			SELECT * FROM post
			ORDER BY created_at DESC
			LIMIT $1 OFFSET $2
		"#,
		paginate.limit(),
		paginate.offset(),
	)
	.fetch_all(&database)
	.await?;

	Ok(Json(posts))
}

/// Get single post
/// Returns a single post by its unique id.
#[route(tag = tag::POST)]
async fn get_one_post(
	State(database): State<Database>,
	Path(post_id): Path<Uuid>,
) -> Result<Json<model::Post>, RouteError> {
	let post = sqlx::query_as!(
		model::Post,
		r#"
			SELECT * FROM post
			WHERE id = $1
		"#,
		post_id,
	)
	.fetch_optional(&database)
	.await?;

	Ok(Json(post.ok_or(Error::UnknownPost(post_id))?))
}
