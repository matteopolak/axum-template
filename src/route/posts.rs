use axum::{
	body::Body,
	extract::{Path, State},
	http::{Response, StatusCode},
	response::IntoResponse,
	routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
	extract::{Json, Query, Session},
	model, AppState, Database,
};

pub fn routes() -> axum::Router<AppState> {
	axum::Router::new()
		.route("/", get(get_all_posts))
		.route("/me", post(get_user_posts))
		.route("/:id", post(get_one_post))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("unknown post {0}")]
	UnknownPost(Uuid),
}

impl Error {
	pub fn status(&self) -> StatusCode {
		match self {
			Self::UnknownPost(..) => StatusCode::NOT_FOUND,
		}
	}
}

impl IntoResponse for Error {
	fn into_response(self) -> Response<Body> {
		crate::Error::from(self).into_response()
	}
}

/// These can be removed when [`serde`] supports
/// literal defaults: <https://github.com/serde-rs/serde/issues/368>
fn one() -> i64 {
	1
}

fn ten() -> i64 {
	10
}

#[derive(Deserialize, Validate)]
pub struct PostPaginateInput {
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "one")]
	pub page: i64,
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "ten")]
	pub size: i64,
}

/// Returns a paginated response of a user's posts,
/// newest first.
async fn get_user_posts(
	State(database): State<Database>,
	session: Session,
	Query(paginate): Query<PostPaginateInput>,
) -> Result<impl IntoResponse, crate::Error> {
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

/// Returns a paginated response of posts, newest first.
async fn get_all_posts(
	State(database): State<Database>,
	Query(paginate): Query<PostPaginateInput>,
) -> Result<impl IntoResponse, crate::Error> {
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

/// Returns a single post by its unique id.
async fn get_one_post(
	State(database): State<Database>,
	Path(post_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::Error> {
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

	match post {
		Some(post) => Ok(Json(post)),
		None => Err(Error::UnknownPost(post_id).into()),
	}
}
