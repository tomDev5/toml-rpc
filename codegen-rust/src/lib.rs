use std::path::Path;
use std::{fs::File, io::Write, path::PathBuf};

use enum_tokens::{TomlRpcEnum, TomlRpcEnumField};
use heck::{ToPascalCase, ToSnakeCase};
use message_tokens::{TomlRpcMessage, TomlRpcMessageField};
use prettyplease::unparse;
use proc_macro2::TokenStream;
use service_tokens::{TomlRpcService, TomlRpcServiceMethod};
use syn::parse_quote;
use toml::{Table, Value};

pub mod enum_tokens;
pub mod message_tokens;
pub mod service_tokens;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Type error: {0}")]
    Types(&'static str),
    #[error("Invalid out dir path")]
    OutDir,
}

pub struct Builder {
    toml_file: PathBuf,
}

impl Builder {
    pub fn new(toml_file: impl AsRef<Path>) -> Self {
        Self {
            toml_file: toml_file.as_ref().to_path_buf(),
        }
    }

    pub fn compile_to_out_dir(self, out_dir: impl AsRef<Path>) -> Result<(), Error> {
        let file_name = self.toml_file.file_name().ok_or(Error::OutDir)?;
        let out_path = out_dir.as_ref().join(file_name).with_extension("rs");
        let mut writer = File::create(&out_path)?;

        self.compile_to_writer(&mut writer)
    }

    pub fn compile_to_writer(self, writer: &mut impl Write) -> Result<(), Error> {
        let generated_code = self.generate_code()?;
        let formatted_code = self.format_code(generated_code)?;
        write!(writer, "{}", formatted_code)?;
        Ok(())
    }

    fn generate_code(&self) -> Result<TokenStream, Error> {
        let toml_file_string = std::fs::read_to_string(&self.toml_file)?;
        let parsed_toml = toml_file_string.parse::<Table>()?;

        let mut tokens = TokenStream::new();

        let messages = parsed_toml
            .get("message")
            .and_then(Value::as_table)
            .cloned()
            .unwrap_or_else(|| Table::default());
        let messages = self.generate_messages(messages.clone())?;
        tokens.extend(
            messages
                .clone()
                .into_iter()
                .map(TomlRpcMessage::into_token_stream),
        );

        let enums = parsed_toml
            .get("enum")
            .and_then(Value::as_table)
            .cloned()
            .unwrap_or_else(|| Table::default());
        let enums = self.generate_enums(enums.clone())?;
        tokens.extend(
            enums
                .clone()
                .into_iter()
                .map(TomlRpcEnum::into_token_stream),
        );

        // Generate traits from RPC
        if let Some(rpc) = parsed_toml.get("rpc").and_then(Value::as_table) {
            tokens.extend(
                self.generate_services(rpc, &messages, &enums)?
                    .into_iter()
                    .map(TomlRpcService::into_token_stream),
            );
        }

        Ok(tokens)
    }

    fn generate_messages(
        &self,
        messages: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<Vec<TomlRpcMessage>, Error> {
        messages
            .into_iter()
            .map(|(message_name, fields)| -> Result<_, Error> {
                let fields = fields
                    .as_table()
                    .ok_or(Error::Types("message is not a table"))?
                    .into_iter()
                    .map(|(tag, data)| -> Result<_, Error> {
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

                        Ok((tag, field_name, field_type))
                    })
                    .collect::<Result<Vec<_>, Error>>()?
                    .into_iter()
                    .map(|(tag, name, field_type)| TomlRpcMessageField::new(tag, name, field_type))
                    .collect::<Vec<_>>();
                Ok(TomlRpcMessage::new(message_name.to_string(), fields))
            })
            .collect::<Result<Vec<TomlRpcMessage>, Error>>()
    }

    fn generate_enums(
        &self,
        enums: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<Vec<TomlRpcEnum>, Error> {
        enums
            .into_iter()
            .map(|(enum_name, variants)| -> Result<_, Error> {
                let fields = variants
                    .as_table()
                    .ok_or(Error::Types("enum is not a table"))?
                    .into_iter()
                    .map(|(variant, numerical_value)| -> Result<_, Error> {
                        let numerical_value: u32 = numerical_value
                            .as_integer()
                            .ok_or(Error::Types("field value is not an array"))?
                            .try_into()
                            .map_err(|_| Error::Types("enum variant value must be a u32"))?;

                        Ok((variant, numerical_value))
                    })
                    .collect::<Result<Vec<_>, Error>>()?
                    .into_iter()
                    .map(|(variant, numerical_value)| {
                        TomlRpcEnumField::new(variant.to_pascal_case(), numerical_value)
                    })
                    .collect::<Vec<_>>();
                Ok(TomlRpcEnum::new(enum_name.to_pascal_case(), fields))
            })
            .collect::<Result<Vec<TomlRpcEnum>, Error>>()
    }

    fn generate_services<'a>(
        &self,
        services: impl IntoIterator<Item = (&'a String, &'a Value)>,
        messages: &[TomlRpcMessage],
        enums: &[TomlRpcEnum],
    ) -> Result<Vec<TomlRpcService>, Error> {
        services
            .into_iter()
            .map(|(service_name, methods)| -> Result<_, Error> {
                let methods = methods
                    .as_table()
                    .ok_or(Error::Types("rpc is not a table"))?
                    .into_iter()
                    .map(|(method_name, method_values)| -> Result<_, Error> {
                        let method_values = method_values
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
                        let (input_type, input_name) = method_input.split_once('.').ok_or(
                            Error::Types("method input must be in the form <message/enum>.name"),
                        )?;
                        match input_type {
                            "message" => {
                                messages
                                    .iter()
                                    .find(|message| message.struct_name == input_name)
                                    .ok_or(Error::Types("message not found"))?;
                            }
                            "enum" => {
                                enums
                                    .iter()
                                    .find(|enum_| enum_.enum_name == input_name)
                                    .ok_or(Error::Types("enum not found"))?;
                            }
                            _ => return Err(Error::Types("unknown input type")),
                        }

                        let (output_type, output_name) = method_output.split_once('.').ok_or(
                            Error::Types("method output must be in the form <message/enum>.name"),
                        )?;
                        match output_type {
                            "message" => {
                                messages
                                    .iter()
                                    .find(|message| message.struct_name == output_name)
                                    .ok_or(Error::Types("message not found"))?;
                            }
                            "enum" => {
                                enums
                                    .iter()
                                    .find(|enum_| enum_.enum_name == output_name)
                                    .ok_or(Error::Types("enum not found"))?;
                            }
                            _ => return Err(Error::Types("unknown output type")),
                        }
                        Ok((
                            method_name.to_snake_case(),
                            input_name.to_pascal_case(),
                            output_name.to_pascal_case(),
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?
                    .into_iter()
                    .map(|(method_name, method_input, method_output)| {
                        TomlRpcServiceMethod::new(method_name, method_input, method_output)
                    })
                    .collect::<Vec<_>>();
                Ok(TomlRpcService::new(service_name.to_pascal_case(), methods))
            })
            .collect::<Result<Vec<TomlRpcService>, Error>>()
    }

    fn format_code(&self, tokens: TokenStream) -> Result<String, Error> {
        let syntax_tree: syn::File = parse_quote! {
            #tokens
        };

        let formatted_code = unparse(&syntax_tree);
        Ok(formatted_code)
    }
}
