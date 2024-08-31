use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use toml::{map::Map, Value};

use crate::Error;

#[derive(Debug, Clone)]
pub struct TomlRpcEnum {
    pub name: String,
    fields: Vec<TomlRpcEnumField>,
}

impl TomlRpcEnum {
    pub fn new(name: String, fields: Vec<TomlRpcEnumField>) -> TomlRpcEnum {
        TomlRpcEnum { name, fields }
    }

    pub fn from_toml(name: String, variants: Map<String, Value>) -> Result<TomlRpcEnum, Error> {
        let fields = variants
            .into_iter()
            .map(|(variant, numerical_value)| TomlRpcEnumField::from_toml(variant, numerical_value))
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(TomlRpcEnum::new(name.to_pascal_case(), fields))
    }

    pub fn into_token_stream(self) -> TokenStream {
        let struct_ident = format_ident!("{}", self.name);
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

    pub fn from_toml(name: String, numerical_value: Value) -> Result<TomlRpcEnumField, Error> {
        let numerical_value: u32 = numerical_value
            .as_integer()
            .ok_or(Error::Types("enum variant value is not an integer"))?
            .try_into()
            .map_err(|_| Error::Types("enum variant value must be a u32"))?;

        Ok(TomlRpcEnumField::new(
            name.to_pascal_case(),
            numerical_value,
        ))
    }

    pub fn into_token_stream(self) -> TokenStream {
        let field_ident = format_ident!("{}", self.name);
        let numerical_value = self.numerical_value;
        quote! {
            #field_ident = #numerical_value,
        }
    }
}
