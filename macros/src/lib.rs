mod model;
mod route;

use proc_macro::TokenStream;

/// Creates a new documentation function for the route, named after the original function with the suffix `_docs`.
#[proc_macro_attribute]
pub fn route(args: TokenStream, input: TokenStream) -> TokenStream {
	route::from_input(args, input)
}

/// Creates two new structs: `CreateX` and `UpdateX` for the model.
/// For both models, fields with #[serde(skip_deserializing)] are skipped, and all
/// other fields are included verbatim (including attributes). Attributes on the struct
/// itself are also copied over to the generated structs.
///
/// # Examples
///
/// ```rust
/// #[model]
/// struct User {
///   #[serde(skip_deserializing)]
///   id: Uuid,
///   name: String,
///   email: String,
/// }
///
/// // Generates:
///
/// struct CreateUser {
///   name: String,
///   email: String,
/// }
///
/// struct UpdateUser {
///   name: Option<String>,
///   email: Option<String>,
/// }
/// ```
#[proc_macro_attribute]
pub fn model(_args: TokenStream, input: TokenStream) -> TokenStream {
	model::from_input(input)
}
