use darling::{ast, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};

#[derive(FromMeta)]
struct RouteArgs {
	#[darling(multiple)]
	tag: Vec<syn::Expr>,
	#[darling(multiple)]
	response: Vec<ResponseArgs>,
}

#[derive(FromMeta)]
struct ResponseArgs {
	status: syn::LitInt,
	shape: Option<syn::Type>,
	description: Option<String>,
}

pub fn from_input(args: TokenStream, input: TokenStream) -> TokenStream {
	let args = match ast::NestedMeta::parse_meta_list(args.into()) {
		Ok(x) => x,
		Err(e) => return e.into_compile_error().into(),
	};

	let args = match RouteArgs::from_list(&args) {
		Ok(x) => x,
		Err(e) => return e.write_errors().into(),
	};

	let function = syn::parse_macro_input!(input as syn::ItemFn);
	let (summary, description) = extract_doc_comment(&function.attrs);

	let fn_name = format_ident!("{}_docs", function.sig.ident);
	let fn_vis = &function.vis;

	let tags = args.tag.iter();
	let responses = args.response.into_iter().map(|response| {
		let status = response.status;
		let shape = response.shape.map_or_else(|| quote!(()), |x| quote!(#x));
		let description = response.description;

		if let Some(description) = description {
			quote! {
				.response_with::<#status, #shape, _>(|res| res.description(#description))
			}
		} else {
			quote! {
				.response::<#status, #shape>()
			}
		}
	});

	quote! {
		#function

		#fn_vis fn #fn_name(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
			op.description(#description).summary(#summary)
				#(
					.tag(#tags)
				)*
				#(
					#responses
				)*
		}
	}
	.into()
}

fn extract_doc_comment(attrs: &[syn::Attribute]) -> (String, String) {
	let mut doc_lines = String::new();
	for attr in attrs {
		if let syn::Meta::NameValue(doc_attr) = &attr.meta {
			if doc_attr.path == quote::format_ident!("doc").into() {
				if let syn::Expr::Lit(lit_expr) = &doc_attr.value {
					if let syn::Lit::Str(literal) = &lit_expr.lit {
						doc_lines += literal.value().trim(); // Trim lines like rustdoc does
						doc_lines += "\n";
					}
				}
			}
		}
	}

	let doc_lines = doc_lines.trim().replace("\\\n", "");
	let mut paragraphs = doc_lines.splitn(2, "\n").filter(|x| !x.is_empty());

	let summary = paragraphs
		.next()
		.map(|x| x.replace("\n", " "))
		.expect("missing description");
	let description = paragraphs
		.next()
		.map(|x| x.to_owned())
		.expect("missing summary");

	(summary, description)
}
