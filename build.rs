#![feature(is_some_and)]

use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fs::{self, read_to_string},
    path::PathBuf,
};

use anyhow::Context;
use either::Either::{self, Left};
use lazy_regex::regex_is_match;
use proc_macro2::Ident;
use quote::{format_ident, quote, ToTokens};
use serde::Deserialize;

fn default_empty() -> Either<Vec<u8>, String> {
    Left(vec![])
}

#[derive(Debug, Deserialize)]
struct IOExample {
    #[serde(default = "default_empty", with = "either::serde_untagged")]
    r#in: Either<Vec<u8>, String>,
    #[serde(with = "either::serde_untagged")]
    out: Either<Vec<u8>, String>,
}

static ENGINES: &[(&str, &str)] = &[
    ("raw", "bf::engine::raw::Engine"),
    ("ir", "bf::engine::ir::Engine"),
];

fn test_fns() -> proc_macro2::TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();
    for (name, path) in ENGINES {
        let name = format_ident!("engine_{}", name);
        let path = syn::parse_str::<syn::Path>(path).unwrap();

        quote!(
            #[test]
            fn #name () {
                super::super::test_engine::<#path>(super::CODE, super::super::IOExample {input: INPUT, output: OUTPUT})
            }
        ).to_tokens(&mut tokens)
    }
    tokens
}

fn bench_fns(source: &str, io_example: &str) -> proc_macro2::TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();
    for (name, path) in ENGINES {
        let fn_name = format_ident!("engine_{}", name);
        let path = syn::parse_str::<syn::Path>(path).unwrap();

        quote!(
            pub fn #fn_name (c: &mut criterion::Criterion) {
                super::super::bench_engine::<#path>(c, #source, #io_example, #name, super::CODE, INPUT)
            }
        ).to_tokens(&mut tokens)
    }
    tokens
}

#[derive(Debug, Clone, Copy)]
struct AsTest<T>(T);
#[derive(Debug, Clone, Copy)]
struct AsBench<T>(T);

struct Example {
    name: String,
    code: String,
    io: HashMap<Ident, IOExample>,
}
impl ToTokens for AsTest<&Example> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let code = &self.0.code;
        quote!(
            static CODE: &str = #code;
        )
        .to_tokens(tokens);
        for (name, IOExample { r#in, out }) in &self.0.io {
            let [r#in, out] = [r#in, out].map(|b| {
                b.as_ref()
                    .map_either(Vec::as_slice, String::as_bytes)
                    .into_inner()
            });
            let tests = test_fns();
            quote!(
                mod #name {
                    static INPUT: &[u8] = &[#(# r#in),*];
                    static OUTPUT: &[u8] = &[#(# out),*];

                    #tests
                }
            )
            .to_tokens(tokens)
        }
    }
}
impl ToTokens for AsBench<&Example> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let code = &self.0.code;
        quote!(
            static CODE: &str = #code;
        )
        .to_tokens(tokens);
        for (name, IOExample { r#in, .. }) in &self.0.io {
            let r#in = r#in
                .as_ref()
                .map_either(Vec::as_slice, String::as_bytes)
                .into_inner();
            let benches = bench_fns(&self.0.name, &name.to_string());
            quote!(
                pub mod #name {
                    static INPUT: &[u8] = &[#(# r#in),*];

                    #benches
                }
            )
            .to_tokens(tokens)
        }
    }
}

struct Examples(HashMap<Ident, Example>);
impl ToTokens for AsTest<&Examples> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for (name, example) in &self.0 .0 {
            let example = AsTest(example);
            quote!(
                mod #name {
                    #example
                }
            )
            .to_tokens(tokens)
        }
    }
}
impl ToTokens for AsBench<&Examples> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for (name, example) in &self.0 .0 {
            let example = AsBench(example);
            quote!(
                pub mod #name {
                    #example
                }
            )
            .to_tokens(tokens);
            let examples = example.0.io.iter().flat_map(|(example, _)| {
                ENGINES.into_iter().map(move |(engine, _)| {
                    let engine = format_ident!("engine_{}", engine);
                    quote!(#name::#example::#engine)
                })
            });
            quote!(criterion_group!(#name, #(#examples),*);).to_tokens(tokens);
        }
        let names = self.0 .0.iter().map(|(n, _)| n);
        quote!(criterion_main!(#(#names),*);).to_tokens(tokens)
    }
}

fn main() -> anyhow::Result<()> {
    let examples = list_examples().context("While reading examples")?;
    tests(&examples)?;
    benches(&examples)?;
    Ok(())
}

fn tests(examples: &Examples) -> anyhow::Result<()> {
    let examples = AsTest(examples);

    let file = PathBuf::from(env::var_os("OUT_DIR").unwrap())
        .join("tests")
        .join("examples.rs");
    fs::create_dir_all(file.parent().unwrap())?;

    let code = quote!(
        # examples
    );

    let code = match syn::parse2::<syn::File>(code.clone()) {
        Ok(file) => prettyplease::unparse(&file),
        Err(err) => {
            cargo_emit::warning!("The example code did not parse correctly as file: {}", err);
            code.to_string()
        }
    };

    fs::write(&file, code)?;
    cargo_emit::rustc_env!("TEST_EXAMPLES", "{}", file.display());
    Ok(())
}

fn benches(examples: &Examples) -> anyhow::Result<()> {
    let examples = AsBench(examples);

    let file = PathBuf::from(env::var_os("OUT_DIR").unwrap())
        .join("benches")
        .join("examples.rs");
    fs::create_dir_all(file.parent().unwrap())?;

    let code = quote!(
        # examples
    );

    let code = match syn::parse2::<syn::File>(code.clone()) {
        Ok(file) => prettyplease::unparse(&file),
        Err(err) => {
            cargo_emit::warning!("The example code did not parse correctly as file: {}", err);
            code.to_string()
        }
    };

    fs::write(&file, code)?;
    cargo_emit::rustc_env!("BENCH_EXAMPLES", "{}", file.display());
    Ok(())
}

fn list_examples() -> anyhow::Result<Examples> {
    let examples_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("bf-sources")
        .join("examples");
    cargo_emit::rerun_if_changed!("{}", examples_dir.display());
    let bf_sources_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("bf-sources");

    let mut examples = HashMap::new();

    for example_file in examples_dir.read_dir()? {
        let example_file = example_file?;
        if example_file.path().extension() == Some(OsStr::new("toml"))
            && example_file
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| regex_is_match!(r"^[a-zA-Z_][a-zA-Z0-9_]*$", s))
            && example_file.file_type()?.is_file()
        {
            let source_file = bf_sources_dir
                .join(example_file.file_name())
                .with_extension("b");
            cargo_emit::rerun_if_changed!("{}", source_file.display());

            let name = example_file
                .path()
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let io =
                toml::from_str::<HashMap<String, IOExample>>(&read_to_string(example_file.path())?)
                    .context(format!(
                        "While parsing header of {}",
                        example_file.path().display()
                    ))?
                    .into_iter()
                    .map(|(name, io)| (format_ident!("{}", name), io))
                    .collect();
            let code = read_to_string(source_file)?;

            examples.insert(format_ident!("{}", name), Example { name, io, code });
        }
    }

    Ok(Examples(examples))
}
