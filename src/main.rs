pub const PC: u64 = u64::MAX;
pub const SP: u64 = u64::MAX - 1;

mod urclrs;
mod codegen;
use crate::urclrs::{lexer::*, ast::*};
use std::rc::Rc;

fn main() {
    let src = std::fs::read_to_string("test.urcl").unwrap();
    let tok = lex(&src);
    let ast = gen_ast(tok, Rc::from(src.to_owned()));
    codegen::Codegen::build(&ast.ast);
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
