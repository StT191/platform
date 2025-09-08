#![feature(proc_macro_span)]

use proc_macro::{TokenStream, TokenTree, Literal, Span};
use std::path::{Path, PathBuf};


fn path_literal(path: &Path) -> TokenStream {
    let token: TokenTree = Literal::string(&path.to_str().unwrap()).into();
    token.into()
}


#[proc_macro]
pub fn dir(_input: TokenStream) -> TokenStream {
    path_literal(PathBuf::from(Span::call_site().file()).parent().unwrap())
}


#[proc_macro]
pub fn project_path(_input: TokenStream) -> TokenStream {
    path_literal(&Path::new(".").canonicalize().unwrap())
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

use quote::quote_spanned;
use syn::{parse_macro_input, LitStr};


// get the next token or return error
macro_rules! next {
    ($span:ident, $input:ident) => {
        if let Some(token) = $input.next() {
            #[allow(unused_assignments)]
            let _ = { $span = token.span() }; // make it a stmt to use the attribute
            token
        }
        else {
            return quote_spanned!{$span.into()=>compile_error!("unexpected end of input")}.into()
        }
    }
}


#[proc_macro]
pub fn rel_path(input: TokenStream) -> TokenStream {

    let dir_path = PathBuf::from(Span::call_site().file()).parent().unwrap().to_owned();

    let mut input = input.into_iter();
    let mut span = Span::call_site();

    // parse path
    let path_token = next!(span, input).into();
    let path_str = parse_macro_input!(path_token as LitStr).value();

    if let Some(token) = input.next() {
        return quote_spanned!{token.span().into()=>compile_error!("unexpected token")}.into()
    }

    path_literal(&norm_path(&dir_path.join(path_str)))
}


#[proc_macro]
pub fn canonical_path(input: TokenStream) -> TokenStream {

    let dir_path = PathBuf::from(Span::call_site().file()).parent().unwrap().to_owned();

    let mut input = input.into_iter();
    let mut span = Span::call_site();

    // parse path
    let path_token = next!(span, input).into();
    let path_str = parse_macro_input!(path_token as LitStr).value();

    if let Some(token) = input.next() {
        return quote_spanned!{token.span().into()=>compile_error!("unexpected token")}.into()
    }

    path_literal(&dir_path.join(path_str).canonicalize().unwrap())
}