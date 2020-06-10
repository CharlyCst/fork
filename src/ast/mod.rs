use crate::error::ErrorHandler;

mod ast;
mod parse;
mod scan;
mod tokens;

pub use ast::*;
pub use tokens::*;

pub fn get_ast(code: &str, error_handler: &mut ErrorHandler) -> ast::Program {
    println!("\n/// Scanning ///\n");

    let mut scanner = scan::Scanner::new(code, 0, error_handler);
    let tokens = scanner.scan();

    for token in tokens.iter() {
        print!("{}", token);
    }
    println!("");

    println!("\n/// Parsing ///\n");

    let mut parser = parse::Parser::new(tokens, error_handler);
    let ast_program = parser.parse();
    println!("{}", ast_program);

    error_handler.print_and_exit();

    ast_program
}
