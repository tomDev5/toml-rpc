use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[derive(Debug, Clone)]
pub struct TomlRpcService {
    service_name: String,
    methods: Vec<TomlRpcServiceMethod>,
}

impl TomlRpcService {
    pub fn new(struct_name: String, fields: Vec<TomlRpcServiceMethod>) -> TomlRpcService {
        TomlRpcService {
            service_name: struct_name,
            methods: fields,
        }
    }

    pub fn into_token_stream(self) -> TokenStream {
        let struct_ident = format_ident!("{}", self.service_name);
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

    pub fn into_token_stream(self) -> TokenStream {
        let name = format_ident!("{}", self.name);
        let input = format_ident!("{}", self.input);
        let output = format_ident!("{}", self.output);

        quote! {
            async fn #name(&self, input: #input) -> #output;
        }
    }
}
