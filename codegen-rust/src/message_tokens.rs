use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use toml::{map::Map, Value};

use crate::Error;

#[derive(Debug, Clone)]
pub struct TomlRpcMessage {
    pub name: String,
    fields: Vec<TomlRpcMessageField>,
}

impl TomlRpcMessage {
    pub fn new(name: String, fields: Vec<TomlRpcMessageField>) -> TomlRpcMessage {
        TomlRpcMessage { name, fields }
    }

    pub fn from_toml(name: String, fields: Map<String, Value>) -> Result<TomlRpcMessage, Error> {
        let fields = fields
            .into_iter()
            .map(|(tag, data)| -> Result<_, Error> { TomlRpcMessageField::from_toml(tag, data) })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(TomlRpcMessage::new(name.to_string(), fields))
    }

    pub fn into_token_stream(self) -> TokenStream {
        let struct_ident = format_ident!("{}", self.name);
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

    pub fn from_toml(tag: String, data: Value) -> Result<TomlRpcMessageField, Error> {
        let tag = tag
            .parse::<u32>()
            .map_err(|_| Error::Types("tag is not a number"))?;
        let data = data
            .as_array()
            .ok_or(Error::Types("field value is not an array"))?;
        if data.len() != 2 {
            return Err(Error::Types(
                "field value must be a two element array (name, type)",
            ));
        }
        let field_name = data[0]
            .as_str()
            .ok_or(Error::Types("field name is not a string"))?
            .to_string();
        let field_type = data[1]
            .as_str()
            .ok_or(Error::Types("field type is not a string"))?
            .to_string();

        Ok(TomlRpcMessageField::new(tag, field_name, field_type))
    }

    pub fn into_token_stream(self) -> TokenStream {
        let field_ident = format_ident!("{}", self.name);
        let rust_type = match self.field_type.as_str() {
            "u32" => quote! { u32 },
            "String" => quote! { String },
            _ => quote! { Unknown },
        };
        let tag = format!(" tag: {}", self.tag);
        quote! {
            #[doc = #tag]
            pub #field_ident: #rust_type,
        }
    }
}
