#![feature(let_chains)]
mod ast;
mod codegen;
mod lexer;
mod parse;
mod ty;

use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: mini-rustc <input>");
        eprintln!("Invalid number of arguments");
        exit(1);
    }

    let lexer = lexer::Lexer::new(&args[1]);
    let mut parser = parse::Parser::new(lexer);
    let parse_result = parser.parse_crate();

    let Some(krate) = parse_result else {
        eprintln!("Failed to parse source code");
        exit(1);
    };

    let mut ctxt = ty::Ctxt::new();

    let codegen_result = codegen::codegen(&krate);
    let Ok(()) = codegen_result else {
        eprintln!("Failed to generate assembly");
        exit(1);
    };
}
