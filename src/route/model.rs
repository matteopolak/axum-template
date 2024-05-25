use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

/// These can be removed when [`serde`] supports
/// literal defaults: <https://github.com/serde-rs/serde/issues/368>
fn one() -> i64 {
	1
}

fn ten() -> i64 {
	10
}

#[derive(Deserialize, Validate, JsonSchema)]
pub struct PaginateInput {
	/// The page number to return (1-indexed).
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "one")]
	pub page: i64,
	/// The number of items to return per page.
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "ten")]
	pub size: i64,
}

impl PaginateInput {
	pub fn offset(&self) -> i64 {
		(self.page - 1) * self.size
	}

	pub fn limit(&self) -> i64 {
		self.size
	}
}

#[derive(Deserialize, Validate, JsonSchema)]
pub struct IdInput {
	pub id: Uuid,
}
