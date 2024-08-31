use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[derive(Debug, Clone)]
pub struct TomlRpcMessage {
    pub struct_name: String,
    fields: Vec<TomlRpcMessageField>,
}

impl TomlRpcMessage {
    pub fn new(struct_name: String, fields: Vec<TomlRpcMessageField>) -> TomlRpcMessage {
        TomlRpcMessage {
            struct_name,
            fields,
        }
    }

    pub fn into_token_stream(self) -> TokenStream {
        let struct_ident = format_ident!("{}", self.struct_name);
        let fields = self
            .fields
            .into_iter()
            .map(|field| field.into_token_stream());
        quote! {
            pub struct #struct_ident {
                #(#fields)*
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TomlRpcMessageField {
    tag: u32, // todo use in comment
    name: String,
    field_type: String,
}

impl TomlRpcMessageField {
    pub fn new(tag: u32, name: String, field_type: String) -> TomlRpcMessageField {
        TomlRpcMessageField {
            tag,
            name,
            field_type,
        }
    }

    pub fn into_token_stream(self) -> TokenStream {
        let field_ident = format_ident!("{}", self.name);
        let rust_type = match self.field_type.as_str() {
            "u32" => quote! { u32 },
            "String" => quote! { String },
            _ => quote! { Unknown },
        };
        quote! {
            pub #field_ident: #rust_type,
        }
    }
}
