use crate::error::ErrorHandler;
use crate::opcode;
use crate::parse::{Block, Expression, Function as ForkFunction, Statement, Value};

#[derive(Debug)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
}

pub struct Function {
    pub params: Vec<Type>,
    pub results: Vec<Type>,
    pub type_index: usize, // Used by encode
    pub body: Vec<opcode::Instr>,
    pub export_name: Option<String>,
}

pub struct Compiler {
    error_handler: ErrorHandler,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            error_handler: ErrorHandler::new(),
        }
    }

    pub fn compile(&mut self, funs: Vec<ForkFunction>) -> Vec<Function> {
        let mut wasm_funs = Vec::new();

        for fun in funs.iter() {
            wasm_funs.push(self.function(fun));
        }

        wasm_funs
    }

    fn function(&mut self, fun: &ForkFunction) -> Function {
        let mut params = Vec::new();
        let results = Vec::new();

        for param in fun.params.iter() {
            params.push(Type::I32)
        }

        let export_name = if fun.exported {
            if fun.ident == "Main" {
                Some(String::from("_start")) // WASI main function
            } else {
                if fun.ident == "main" {
                    self.error_handler
                        .report(0, "Main function must be capitalized") // TODO report line
                }
                Some(fun.ident.clone())
            }
        } else {
            None
        };

        let mut opcode = Vec::new();
        opcode.push(0x00); // local count
        self.block(&fun.block, &mut opcode);
        opcode.push(opcode::INSTR_END);

        Function {
            params: params,
            results: results,
            type_index: std::usize::MAX,
            body: opcode,
            export_name: export_name,
        }
    }

    fn block(&mut self, block: &Block, opcode: &mut Vec<opcode::Instr>) {
        for stmt in block.stmts.iter() {
            match stmt {
                Statement::ReturnStmt { expr } => {
                    match expr {
                        Some(e) => self.expression(e, opcode),
                        None => (),
                    };
                    opcode.push(opcode::INSTR_RETURN);
                }
                _ => (),
            }
        }
    }

    fn expression(&mut self, expr: &Expression, opcode: &mut Vec<opcode::Instr>) {
        match expr {
            Expression::Literal {
                value: Value::Number(n),
            } => {
                opcode.push(opcode::INSTR_I32_CST);
                opcode.extend(opcode::to_leb(*n as usize));
            }
            Expression::Literal {
                value: Value::Boolean(_),
            } => self
                .error_handler
                .report(0, "Boolean are not yet supported"),
            _ => self.error_handler.report(0, "Expression not yet supported"),
        }
    }
}
