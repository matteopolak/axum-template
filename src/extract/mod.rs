mod session;

pub use session::{Session, SessionOrApiKey};

use aide::OperationIo;
use axum::{
	body::Body,
	extract::{FromRequest, FromRequestParts, Request},
	http::{request, Response},
	response::IntoResponse,
};
use schemars::JsonSchema;
use serde::de;

use crate::error::AppError;

/// Extractor that deserializes a JSON body and validates it.
///
/// T must implement [`serde::de::DeserializeOwned`] and [`validator::Validate`]
/// in order to be used in an extractor.
///
/// ```rust
/// async fn route(Json(user): Json<User>) {
///   // ...
/// }
/// ```
#[derive(OperationIo)]
#[aide(
	input_with = "axum_jsonschema::Json<T>",
	output_with = "axum_jsonschema::Json<T>",
	json_schema
)]
pub struct Json<T>(pub T);

impl<T> IntoResponse for Json<T>
where
	T: serde::Serialize,
{
	fn into_response(self) -> Response<Body> {
		axum::extract::Json(self.0).into_response()
	}
}

#[axum::async_trait]
impl<T, S> FromRequest<S> for Json<T>
where
	T: de::DeserializeOwned + validator::Validate + JsonSchema + 'static,
	S: Send + Sync,
{
	type Rejection = AppError;

	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
		let result = axum_jsonschema::Json::<T>::from_request(req, state)
			.await?
			.0;

		result.validate().map_err(Self::Rejection::Validation)?;
		Ok(Self(result))
	}
}

/// Extractor that deserializes a query string and validates it.
///
/// This is similar to [`Json<T>`], but does not consume the body.
///
/// ```rust
/// async fn route(Query(params): Query<Params>) {
///   // ...
/// }
/// ```
#[derive(OperationIo)]
#[aide(
	input_with = "axum::extract::Query<T>",
	output_with = "axum_jsonschema::Json<T>",
	json_schema
)]
pub struct Query<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequestParts<S> for Query<T>
where
	T: de::DeserializeOwned + validator::Validate,
	S: Send + Sync,
{
	type Rejection = AppError;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		let result = axum::extract::Query::<T>::from_request_parts(parts, state)
			.await?
			.0;

		result.validate().map_err(Self::Rejection::Validation)?;
		Ok(Self(result))
	}
}

/// Extractor that deserializes a path parameter and validates it.
#[derive(OperationIo)]
#[aide(
	input_with = "axum::extract::Path<T>",
	output_with = "axum_jsonschema::Json<T>",
	json_schema
)]
pub struct Path<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequestParts<S> for Path<T>
where
	T: de::DeserializeOwned + validator::Validate + Send,
	S: Send + Sync,
{
	type Rejection = AppError;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		let result = axum::extract::Path::<T>::from_request_parts(parts, state)
			.await?
			.0;

		result.validate().map_err(Self::Rejection::Validation)?;
		Ok(Self(result))
	}
}
