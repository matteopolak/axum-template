use std::borrow::Cow;

use aide::{
	axum::{
		routing::{get_with, post_with},
		ApiRouter, IntoApiResponse,
	},
	transform::TransformOperation,
};
use axum::{
	extract::{Path, State},
	http::StatusCode,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use crate::{
	error,
	extract::{Json, Query, Session},
	model, AppState, Database,
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

/// These can be removed when [`serde`] supports
/// literal defaults: <https://github.com/serde-rs/serde/issues/368>
fn one() -> i64 {
	1
}

fn ten() -> i64 {
	10
}

#[derive(Deserialize, Validate, JsonSchema)]
pub struct PostPaginateInput {
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "one")]
	pub page: i64,
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "ten")]
	pub size: i64,
}

fn get_user_posts_docs(op: TransformOperation) -> TransformOperation {
	op.summary("Get a user's posts")
		.description("Returns a paginated response of a user's posts, newest first.")
		.tag("posts")
}

/// Returns a paginated response of a user's posts,
/// newest first.
async fn get_user_posts(
	State(database): State<Database>,
	session: Session,
	Query(paginate): Query<PostPaginateInput>,
) -> Result<impl IntoApiResponse, RouteError> {
	let posts = sqlx::query_as!(
		model::Post,
		r#"
    SELECT * FROM post
    WHERE user_id = $1
    ORDER BY created_at DESC
    LIMIT $2 OFFSET $3
    "#,
		session.user.id,
		paginate.size,
		paginate.size * (paginate.page - 1),
	)
	.fetch_all(&database)
	.await?;

	Ok(Json(posts))
}

fn get_all_posts_docs(op: TransformOperation) -> TransformOperation {
	op.summary("Get all posts")
		.description("Returns a paginated response of posts, newest first.")
		.tag("posts")
}

/// Returns a paginated response of posts, newest first.
async fn get_all_posts(
	State(database): State<Database>,
	Query(paginate): Query<PostPaginateInput>,
) -> Result<impl IntoApiResponse, RouteError> {
	let posts = sqlx::query_as!(
		model::Post,
		r#"
    SELECT * FROM post
    ORDER BY created_at DESC
    LIMIT $1 OFFSET $2
    "#,
		paginate.size,
		paginate.size * (paginate.page - 1),
	)
	.fetch_all(&database)
	.await?;

	Ok(Json(posts))
}

fn get_one_post_docs(op: TransformOperation) -> TransformOperation {
	op.summary("Get a single post")
		.description("Returns a single post by its unique id.")
		.tag("posts")
}

/// Returns a single post by its unique id.
async fn get_one_post(
	State(database): State<Database>,
	Path(post_id): Path<Uuid>,
) -> Result<impl IntoApiResponse, RouteError> {
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
