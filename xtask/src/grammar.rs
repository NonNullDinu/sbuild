use crate::format::format_file;

use super::ws_path;
use lexer::Lexer;
use log::*;
use std::error::Error;

use serde::{Deserialize, Serialize};

use quote::quote;

mod ast_src;
mod kinds;
mod lexer;
mod lower;
mod syn_tree_implementation;

#[derive(Serialize, Deserialize, Debug)]
pub struct ConstToken {
    name: String,
    #[serde(rename = "lowercase")]
    lowercased_name: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Token {
    name: String,
    #[serde(rename = "lowercase")]
    lowercased_name: String,
    regex: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Grammar {
    const_tokens: Vec<ConstToken>,
    tokens: Vec<Token>,
}

pub fn generate_grammar() -> Result<(), Box<dyn Error>> {
    let p = ws_path!("crates" / "leafbuild-syntax" / "grammar.ron");
    info!("Generating grammar from {:?}", p);

    let contents = std::fs::read_to_string(p)?;

    let grammar = ron::from_str(&contents)?;
    let Grammar {
        ref const_tokens,
        ref tokens,
    } = grammar;

    let ungrm: ungrammar::Grammar =
        std::fs::read_to_string(ws_path!("crates" / "leafbuild-syntax" / "syntax.ungram"))?
            .parse()
            .unwrap();

    let src = lower::lower(&ungrm);

    let lexer = Lexer {
        tokens,
        const_tokens,
    };

    {
        let s = quote! {#lexer}.to_string();
        let path = ws_path!("crates" / "leafbuild-syntax" / "src" / "lexer.rs");
        std::fs::write(
            &path,
            format!("{}\n{}", "// @generated by xtask generate-grammar", s),
        )?;
        info!("Wrote {:?}, now formatting", &path);

        format_file(&path)?;

        info!("Formatted {:?}", path);
    }

    {
        let s = kinds::generate_kinds(ast_src::KINDS_SRC).to_string();
        let path = ws_path!("crates" / "leafbuild-syntax" / "src" / "syntax_kind.rs");
        std::fs::write(
            &path,
            format!("{}\n{}", "// @generated by xtask generate-grammar", s),
        )?;
        info!("Wrote {:?}, now formatting", &path);

        format_file(&path)?;

        info!("Formatted {:?}", path);
    }

    {
        let s = syn_tree_implementation::generate_nodes(ast_src::KINDS_SRC, &src).to_string();
        let path = ws_path!("crates" / "leafbuild-syntax" / "src" / "ast" / "implementation.rs");
        std::fs::write(
            &path,
            format!("{}\n{}", "// @generated by xtask generate-grammar", s),
        )?;
        info!("Wrote {:?}, now formatting", &path);

        format_file(&path)?;

        info!("Formatted {:?}", path);
    }

    Ok(())
}

fn raw_str_literal(s: &'_ str) -> ::proc_macro2::TokenStream {
    let mut depth = 0;
    let mut cur_depth = None;
    let mut chars = s.chars().peekable();
    loop {
        match (cur_depth, chars.peek()) {
            (_, Some('#')) => {}
            (Some(d), _) => depth = depth.max(d),
            _ => {}
        }
        match (chars.next(), cur_depth) {
            (None, _) => break,
            (Some('"'), _) => cur_depth = Some(0),
            (Some('#'), Some(ref mut d)) => *d += 1,
            (Some(_), _) => cur_depth = None,
        }
    }
    format!(
        "r{0:#^raw_depth$}\"{wrapped}\"{0:#^raw_depth$}",
        "",
        wrapped = s,
        raw_depth = depth + 1,
    )
    .parse()
    .unwrap()
}

fn to_upper_snake_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev {
            buf.push('_')
        }
        prev = true;

        buf.push(c.to_ascii_uppercase());
    }
    buf
}

fn to_lower_snake_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev {
            buf.push('_')
        }
        prev = true;

        buf.push(c.to_ascii_lowercase());
    }
    buf
}

fn to_pascal_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev_is_underscore = true;
    for c in s.chars() {
        if c == '_' {
            prev_is_underscore = true;
        } else if prev_is_underscore {
            buf.push(c.to_ascii_uppercase());
            prev_is_underscore = false;
        } else {
            buf.push(c.to_ascii_lowercase());
        }
    }
    buf
}

fn pluralize(s: &str) -> String {
    format!("{}s", s)
}
