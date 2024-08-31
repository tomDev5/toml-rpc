use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[derive(Debug, Clone)]
pub struct TomlRpcEnum {
    pub enum_name: String,
    fields: Vec<TomlRpcEnumField>,
}

impl TomlRpcEnum {
    pub fn new(struct_name: String, fields: Vec<TomlRpcEnumField>) -> TomlRpcEnum {
        TomlRpcEnum {
            enum_name: struct_name,
            fields,
        }
    }

    pub fn into_token_stream(self) -> TokenStream {
        let struct_ident = format_ident!("{}", self.enum_name);
        let fields = self
            .fields
            .into_iter()
            .map(|field| field.into_token_stream());
        quote! {
            #[repr(u32)]
            pub enum #struct_ident {
                #(#fields)*
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TomlRpcEnumField {
    name: String,
    numerical_value: u32,
}

impl TomlRpcEnumField {
    pub fn new(name: String, numerical_value: u32) -> TomlRpcEnumField {
        TomlRpcEnumField {
            name,
            numerical_value,
        }
    }

    pub fn into_token_stream(self) -> TokenStream {
        let field_ident = format_ident!("{}", self.name);
        let numerical_value = self.numerical_value;
        quote! {
            #field_ident = #numerical_value,
        }
    }
}
