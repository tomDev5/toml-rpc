use std::path::Path;
use std::{fs::File, io::Write, path::PathBuf};

use enum_tokens::TomlRpcEnum;
use message_tokens::TomlRpcMessage;
use prettyplease::unparse;
use proc_macro2::TokenStream;
use service_tokens::TomlRpcService;
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
            .map(|(name, fields)| -> Result<_, Error> {
                TomlRpcMessage::from_toml(
                    name,
                    fields
                        .as_table()
                        .cloned()
                        .ok_or(Error::Types("enum is not a table"))?,
                )
            })
            .collect::<Result<Vec<TomlRpcMessage>, Error>>()
    }

    fn generate_enums(
        &self,
        enums: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<Vec<TomlRpcEnum>, Error> {
        enums
            .into_iter()
            .map(|(name, variants)| {
                TomlRpcEnum::from_toml(
                    name,
                    variants
                        .as_table()
                        .cloned()
                        .ok_or(Error::Types("enum is not a table"))?,
                )
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
                TomlRpcService::from_toml(
                    service_name.to_string(),
                    methods
                        .as_table()
                        .cloned()
                        .ok_or(Error::Types("service is not a table"))?,
                    messages,
                    enums,
                )
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
