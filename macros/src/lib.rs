mod model;
mod route;

use proc_macro::TokenStream;

/// Creates a new documentation function for the route, named after the original function with the suffix `_docs`.
#[proc_macro_attribute]
pub fn route(args: TokenStream, input: TokenStream) -> TokenStream {
	route::from_input(args, input)
}

/// Creates two new structs: `CreateXInput` and `UpdateXInput` for the model.
/// For both models, fields with #[serde(skip_deserializing)] are skipped, and all
/// other fields (excluding `id`) are included verbatim (including attributes).
#[proc_macro_attribute]
pub fn model(_args: TokenStream, input: TokenStream) -> TokenStream {
	model::from_input(input)
}
