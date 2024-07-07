use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

/// These can be removed when [`serde`] supports
/// literal defaults: <https://github.com/serde-rs/serde/issues/368>
#[inline]
fn one() -> i64 {
	1
}

#[inline]
fn ten() -> i64 {
	10
}

#[derive(Deserialize, Validate, JsonSchema)]
pub struct Paginate {
	/// The page number to return (1-indexed).
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "one")]
	pub page: i64,
	/// The number of items to return per page.
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "ten")]
	pub size: i64,
}

impl Paginate {
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

#[cfg(test)]
mod test {
	#[test]
	fn test_paginate_offset() {
		let mut paginate = super::Paginate { page: 1, size: 10 };

		assert_eq!(paginate.offset(), 0);

		paginate.page = 2;

		assert_eq!(paginate.offset(), 10);

		paginate.size = 5;

		assert_eq!(paginate.offset(), 5);

		paginate.page = 3;

		assert_eq!(paginate.offset(), 10);
	}

	#[test]
	fn test_paginate_limit() {
		let paginate = super::Paginate { page: 1, size: 10 };

		assert_eq!(paginate.limit(), 10);
	}
}
