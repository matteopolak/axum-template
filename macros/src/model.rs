use darling::{ast, FromDeriveInput, FromField};
use proc_macro2::TokenTree;
use quote::{format_ident, quote, ToTokens};
use syn::Meta;

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named), forward_attrs)]
struct ModelInputReceiver {
	ident: syn::Ident,

	generics: syn::Generics,

	data: ast::Data<(), ModelFieldReceiver>,

	attrs: Vec<syn::Attribute>,
}

#[derive(Debug, FromField)]
#[darling(forward_attrs)]
struct ModelFieldReceiver {
	ident: Option<syn::Ident>,

	ty: syn::Type,
	vis: syn::Visibility,

	attrs: Vec<syn::Attribute>,
}

pub fn from_input(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = syn::parse_macro_input!(input as syn::DeriveInput);
	let receiver = match ModelInputReceiver::from_derive_input(&input) {
		Ok(x) => x,
		Err(e) => return e.write_errors().into(),
	};

	let ident = &receiver.ident;
	let vis = &input.vis;
	let generics = &receiver.generics;
	let create_ident = format_ident!("Create{}", ident);
	let update_ident = format_ident!("Update{}", ident);

	let attrs = &receiver.attrs;

	let fields = receiver.data.take_struct().expect("expected struct");
	let fields = fields
		.iter()
		.filter_map(|field| {
			let ident = field.ident.as_ref()?;
			let ty = &field.ty;
			let attrs = &field.attrs;
			let vis = &field.vis;

			// Skip fields with #[serde(skip_deserializing)] or #[serde(skip)]
			if attrs.iter().any(|attr| {
				let Meta::List(ref list) = attr.meta else {
					return false;
				};

				if !list.path.is_ident("serde") {
					return false;
				}

				list.tokens.to_token_stream().into_iter().any(|token| {
					matches!(token, TokenTree::Ident(ref ident) if ident == "skip_deserializing" || ident == "skip")
				})
			}) {
				return None;
			}

			Some((attrs, ident, ty, vis))
		})
		.collect::<Vec<_>>();

	let create_fields = fields.iter().map(|(attrs, ident, ty, vis)| {
		quote! {
			#(#attrs)*
			#vis #ident: #ty,
		}
	});

	let update_fields = fields.iter().map(|(attrs, ident, ty, vis)| {
		quote! {
			#(#attrs)*
			#vis #ident: Option<#ty>,
		}
	});

	quote! {
		#input

		#(#attrs)*
		#vis struct #create_ident #generics {
			#(
				#create_fields
			)*
		}

		#(#attrs)*
		#vis struct #update_ident #generics {
			#(
				#update_fields
			)*
		}
	}
	.into()
}
