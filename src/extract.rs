use aide::{openapi, operation, OperationInput, OperationIo};
use axum::{
	body::Body,
	extract::{FromRef, FromRequest, FromRequestParts, Request},
	http::{header, request, Response},
	response::IntoResponse,
};
use schemars::JsonSchema;
use serde::de;
use uuid::Uuid;

use crate::{error::Error, model, route::auth, session, Database};

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
	type Rejection = Error;

	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
		let result = axum_jsonschema::Json::<T>::from_request(req, state)
			.await?
			.0;

		result.validate().map_err(Error::Validation)?;
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
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		let result = axum::extract::Query::<T>::from_request_parts(parts, state)
			.await?
			.0;

		result.validate().map_err(Error::Validation)?;
		Ok(Self(result))
	}
}

/// Extracts the session and related user from the request.
///
/// If it does not exist, a [`auth::Error::NoSessionCookie`] is returned.
/// If the session is invalid, a [`auth::Error::InvalidSessionCookie`] is returned.
///
/// ```rust
/// async fn route(session: Session) {
///   println!("{:?}", session.user);
/// }
/// ```
#[derive(Debug)]
pub struct Session {
	pub id: Uuid,
	pub user: model::User,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for Session
where
	Database: FromRef<S>,
	S: Sync + Send,
{
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		let cookies = parts
			.headers
			.get_all(header::COOKIE)
			.into_iter()
			.filter_map(|value| value.to_str().ok());

		let session_id = cookies
			.flat_map(cookie::Cookie::split_parse)
			.filter_map(Result::ok)
			.find(|cookie| cookie.name() == session::COOKIE_NAME)
			.ok_or(auth::Error::NoSessionCookie)?;

		let session_id =
			Uuid::parse_str(session_id.value()).map_err(|_| auth::Error::InvalidSessionCookie)?;

		let database = Database::from_ref(state);
		let user = sqlx::query_as!(
			model::User,
			r#"
				SELECT * FROM "user" WHERE id = (
					SELECT user_id FROM session WHERE id = $1
				)
			"#,
			session_id
		)
		.fetch_optional(&database)
		.await?;

		let Some(user) = user else {
			return Err(auth::Error::InvalidSessionCookie.into());
		};

		Ok(Self {
			user,
			id: session_id,
		})
	}
}

impl OperationInput for Session {
	/// Operation input for the session extractor.
	///
	/// This adds a session cookie requirement to the `OpenAPI` operation.
	fn operation_input(ctx: &mut aide::gen::GenContext, operation: &mut aide::openapi::Operation) {
		let s = ctx.schema.subschema_for::<Uuid>();

		operation::add_parameters(
			ctx,
			operation,
			[openapi::Parameter::Cookie {
				parameter_data: openapi::ParameterData {
					name: session::COOKIE_NAME.to_string(),
					required: true,
					description: Some("The session cookie for the current user.".to_string()),
					format: openapi::ParameterSchemaOrContent::Schema(openapi::SchemaObject {
						json_schema: s,
						example: None,
						external_docs: None,
					}),
					extensions: Default::default(),
					deprecated: Some(false),
					example: None,
					examples: Default::default(),
					explode: None,
				},
				style: openapi::CookieStyle::Form,
			}],
		);
	}
}
