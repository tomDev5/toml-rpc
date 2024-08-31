use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use toml::{map::Map, Value};

use crate::{enum_tokens::TomlRpcEnum, message_tokens::TomlRpcMessage, Error};

#[derive(Debug, Clone)]
pub struct TomlRpcService {
    name: String,
    methods: Vec<TomlRpcServiceMethod>,
}

impl TomlRpcService {
    pub fn new(name: String, methods: Vec<TomlRpcServiceMethod>) -> TomlRpcService {
        TomlRpcService { name, methods }
    }

    pub fn from_toml(
        name: String,
        services: Map<String, Value>,
        messages: &[TomlRpcMessage],
        enums: &[TomlRpcEnum],
    ) -> Result<TomlRpcService, Error> {
        let services = services
            .into_iter()
            .map(|(tag, data)| -> Result<_, Error> {
                TomlRpcServiceMethod::from_toml(tag, data, messages, enums)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(TomlRpcService::new(name.to_string(), services))
    }

    pub fn into_token_stream(self) -> TokenStream {
        let struct_ident = format_ident!("{}", self.name);
        let fields = self
            .methods
            .into_iter()
            .map(|field| field.into_token_stream());
        quote! {
            pub trait #struct_ident {
                #(#fields)*
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TomlRpcServiceMethod {
    name: String,
    input: String,
    output: String,
}

impl TomlRpcServiceMethod {
    pub fn new(name: String, input: String, output: String) -> TomlRpcServiceMethod {
        TomlRpcServiceMethod {
            name,
            input,
            output,
        }
    }

    pub fn from_toml(
        name: String,
        values: Value,
        messages: &[TomlRpcMessage],
        enums: &[TomlRpcEnum],
    ) -> Result<TomlRpcServiceMethod, Error> {
        let method_values = values
            .as_array()
            .ok_or(Error::Types("field value is not an array"))?;
        if method_values.len() != 2 {
            return Err(Error::Types(
                "field value must be a two element array (name, type)",
            ));
        }
        let method_input = method_values[0]
            .as_str()
            .ok_or(Error::Types("method input is not a string"))?
            .to_string();
        let method_output = method_values[1]
            .as_str()
            .ok_or(Error::Types("method output is not a string"))?
            .to_string();

        // verify that input and output exist:
        let (input_type, input_name) = method_input.split_once('.').ok_or(Error::Types(
            "method input must be in the form <message/enum>.name",
        ))?;
        match input_type {
            "message" => {
                messages
                    .iter()
                    .find(|message| message.name == input_name)
                    .ok_or(Error::Types("message not found"))?;
            }
            "enum" => {
                enums
                    .iter()
                    .find(|enum_| enum_.name == input_name)
                    .ok_or(Error::Types("enum not found"))?;
            }
            _ => return Err(Error::Types("unknown input type")),
        }

        let (output_type, output_name) = method_output.split_once('.').ok_or(Error::Types(
            "method output must be in the form <message/enum>.name",
        ))?;
        match output_type {
            "message" => {
                messages
                    .iter()
                    .find(|message| message.name == output_name)
                    .ok_or(Error::Types("message not found"))?;
            }
            "enum" => {
                enums
                    .iter()
                    .find(|enum_| enum_.name == output_name)
                    .ok_or(Error::Types("enum not found"))?;
            }
            _ => return Err(Error::Types("unknown output type")),
        }

        Ok(TomlRpcServiceMethod::new(
            name.to_snake_case(),
            input_name.to_pascal_case(),
            output_name.to_pascal_case(),
        ))
    }

    pub fn into_token_stream(self) -> TokenStream {
        let name = format_ident!("{}", self.name);
        let input = format_ident!("{}", self.input);
        let output = format_ident!("{}", self.output);

        quote! {
            async fn #name(&self, input: #input) -> #output;
        }
    }
}
