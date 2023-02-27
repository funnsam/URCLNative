pub const PC: u64 = u64::MAX;
pub const SP: u64 = u64::MAX - 1;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod urclrs;
mod codegen;
mod args;
use crate::{urclrs::{lexer::*, ast::*}, args::*};
use std::{rc::Rc, process::exit};

fn main() {
    let arg = match Args::parse() {
        ParseResult::Ok(a) => a,
        ParseResult::Err(err) => {
            println!("\x1b[1;31mError: {err}\x1b[0m");
            exit(-1);
        },
        ParseResult::Help(msg) => {
            println!("{msg}");
            exit(0);
        }
    };
    let src = match std::fs::read_to_string(&arg.file) {
        Ok(a) => a,
        Err(a) => {
            println!("\x1b[1;31mError: Couldn't read \"{}\": {}\x1b[0m", &arg.file, a);
            exit(-1);
        }
    };
    let tok = lex(&src);
    let ast = gen_ast(tok, Rc::from(src.to_owned()));
    if ast.err.has_error() {
        println!("{}", ast.err.to_string(&src));
        return;
    }
    codegen::Codegen::build(&ast.ast, &arg.output, arg.debug, &arg.file);
}

pub fn out_err(out: &mut String, error: &urclrs::errorcontext::Error, lineno: &String, line: &str, col: usize) {
    use std::fmt::Write;
    use crate::urclrs::errorcontext::*;
    writeln!(out, "\x1b[1;{}m{}: {}\x1b[0;0m",
        match error.level {
            ErrorLevel::Info    => 36,
            ErrorLevel::Warning => 33,
            ErrorLevel::Error   => 31,
        }, error.level, error.kind
    ).unwrap();
    writeln!(out, "\t{}| {}", 
        lineno, &line.split_at(get_indent_level(line)).1.replace("\t", " ")
    ).unwrap();
    writeln!(out, "\t{}| {}{}",
        " ".repeat(str_width(lineno)),
        &" ".repeat(col - get_indent_level(line)),
        &"^".repeat(str_width(error.span).max(1))
    ).unwrap();
}
