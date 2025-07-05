//! # Rustash Macros
//!
//! Procedural macros for the Rustash project.

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for snippet metadata
#[proc_macro_derive(SnippetMetadata)]
pub fn snippet_metadata(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let expanded = quote! {
        impl SnippetMetadata for #name {
            fn get_metadata(&self) -> std::collections::HashMap<String, String> {
                std::collections::HashMap::new()
            }
        }
    };
    
    TokenStream::from(expanded)
}