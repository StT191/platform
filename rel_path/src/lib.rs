#![feature(proc_macro_expand)]

use proc_macro::{TokenStream, TokenTree, Literal, Span};
use std::path::{Path, PathBuf};

use quote::{quote_spanned, quote};
use syn::{parse_macro_input, LitStr};


// helper

fn dir_path() -> PathBuf {
    PathBuf::from(Span::call_site().file()).parent().unwrap().to_owned()
}

fn norm_path(path: &Path) -> PathBuf {

    let mut normal = PathBuf::new();
    let mut level: usize = 0;

    for part in path.iter() {
        if part == ".." {
            if level != 0 { normal.pop(); level -= 1 }
            else { normal.push(".."); }
        }
        else if part != "." {
            normal.push(part);
            level += 1;
        }
    }

    normal
}

fn path_literal(path: &Path) -> TokenStream {
    let token: TokenTree = Literal::string(&path.to_str().unwrap()).into();
    token.into()
}


macro_rules! parse_input_path {
    ($input:expr) => {{
        let mut input = $input.into_iter();

        // parse path
        let path_token = if let Some(token) = input.next() { token.into() }
        else {
            return quote_spanned!{Span::call_site().into()=>compile_error!("unexpected end of input")}.into()
        };

        let path_string = parse_macro_input!(path_token as LitStr).value();

        if let Some(token) = input.next() {
            return quote_spanned!{token.span().into()=>compile_error!("unexpected token")}.into()
        }

        path_string
    }}
}


// macros

#[proc_macro]
pub fn dir(_input: TokenStream) -> TokenStream {
    path_literal(PathBuf::from(Span::call_site().file()).parent().unwrap())
}


#[proc_macro]
pub fn project_path(_input: TokenStream) -> TokenStream {
    path_literal(&Path::new(".").canonicalize().unwrap())
}


#[proc_macro]
pub fn rel_path(input: TokenStream) -> TokenStream {
    let path_string = parse_input_path!(input);
    path_literal(&norm_path(&dir_path().join(path_string)))
}


#[proc_macro]
pub fn canonical_path(input: TokenStream) -> TokenStream {
    let path_string = parse_input_path!(input);
    path_literal(&dir_path().join(path_string).canonicalize().unwrap())
}


#[proc_macro]
pub fn __expand_as_compile_error(input: TokenStream) -> TokenStream {
    let input_str = input.expand_expr().unwrap().to_string();
    return quote!{compile_error!(#input_str)}.into()
}