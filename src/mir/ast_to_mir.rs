use super::mir::*;
use super::names::{
    AsmControl, AsmParametric, AsmStatement, Block as NameBlock, Body as NameBody,
    Expression as Expr, Function as NameFun, NameStore, Statement as S, Value as V,
};
use super::types::{Type as ASTTypes, TypeId, TypeStore};
use super::TypedProgram;

use crate::ast::{BinaryOperator as ASTBinop, UnaryOperator as ASTUnop};
use crate::error::ErrorHandler;
use std::convert::TryInto;

enum FromBinop {
    Binop(Binop),
    Relop(Relop),
    Logical(Logical),
}

struct State {
    pub names: NameStore,
    pub types: TypeStore,
    bb_id: BasicBlockId,
}

impl State {
    pub fn new(names: NameStore, types: TypeStore) -> State {
        State {
            names: names,
            types: types,
            bb_id: 0,
        }
    }

    pub fn fresh_bb_id(&mut self) -> BasicBlockId {
        let id = self.bb_id;
        self.bb_id += 1;
        id
    }
}

pub struct MIRProducer<'a> {
    err: &'a mut ErrorHandler,
}

impl<'a> MIRProducer<'a> {
    pub fn new(error_handler: &mut ErrorHandler) -> MIRProducer {
        MIRProducer { err: error_handler }
    }

    /// Lower a typed program to MIR
    pub fn reduce(&mut self, prog: TypedProgram) -> Program {
        let mut state = State::new(prog.names, prog.types);
        let mut funs = Vec::with_capacity(prog.funs.len());

        for fun in prog.funs.into_iter() {
            match self.reduce_fun(fun, &mut state) {
                Ok(fun) => funs.push(fun),
                Err(err) => self.err.report_internal_no_loc(err),
            }
        }

        Program {
            funs: funs,
            pub_decls: prog.pub_decls,
        }
    }

    fn reduce_fun(&mut self, fun: NameFun, s: &mut State) -> Result<Function, String> {
        let fun_name = s.names.get(fun.n_id);
        let (param_t, ret_t) = if let ASTTypes::Fun(param_t, ret_t) = s.types.get(fun_name.t_id) {
            let param_t: Result<Vec<Type>, String> =
                param_t.into_iter().map(|t| convert_type(t)).collect();
            let ret_t: Result<Vec<Type>, String> =
                ret_t.into_iter().map(|t| convert_type(t)).collect();
            (param_t?, ret_t?)
        } else {
            self.err.report_internal(
                fun.loc,
                String::from("Function does not have function type"),
            );
            (vec![], vec![])
        };

        let params = fun.params.iter().map(|p| p.n_id).collect();
        let locals = self.get_locals(&fun, s)?;
        let block = match fun.body {
            NameBody::Zephyr(block) => self.reduce_block(block, s)?,
            NameBody::Asm(stmts) => Block::Block {
                id: s.fresh_bb_id(),
                stmts: self.reduce_asm_statements(stmts, s)?,
                t: None,
            },
        };

        Ok(Function {
            ident: fun.ident,
            params: params,
            param_types: param_t,
            ret_types: ret_t,
            locals: locals,
            body: block,
            is_pub: fun.is_pub,
            exposed: fun.exposed,
            fun_id: fun.fun_id,
        })
    }

    fn get_locals(&mut self, fun: &NameFun, s: &State) -> Result<Vec<Local>, String> {
        let mut locals = Vec::new();
        for local_name in &fun.locals {
            let t_id = s.names.get(*local_name).t_id;
            let t = match s.types.get(t_id) {
                ASTTypes::I32 => Type::I32,
                ASTTypes::I64 => Type::I64,
                ASTTypes::F32 => Type::F32,
                ASTTypes::F64 => Type::F64,
                ASTTypes::Bool => Type::I32,
                _ => return Err(format!("Invalid parameter type for t_id {}", t_id)),
            };
            locals.push(Local {
                id: *local_name,
                t: t,
            })
        }

        Ok(locals)
    }

    fn reduce_block(&mut self, block: NameBlock, s: &mut State) -> Result<Block, String> {
        let id = s.fresh_bb_id();
        let mut stmts = Vec::new();
        self.reduce_block_rec(block, &mut stmts, s)?;
        let reduced_block = Block::Block {
            id: id,
            stmts: stmts,
            t: None,
        };
        Ok(reduced_block)
    }

    fn reduce_block_rec(
        &mut self,
        block: NameBlock,
        stmts: &mut Vec<Statement>,
        s: &mut State,
    ) -> Result<(), String> {
        for statement in block.stmts.into_iter() {
            match statement {
                S::AssignStmt { var, expr } => {
                    self.reduce_expr(&expr, stmts, s)?;
                    stmts.push(Statement::Set { l_id: var.n_id });
                }
                S::LetStmt { var, expr } => {
                    self.reduce_expr(&expr, stmts, s)?;
                    stmts.push(Statement::Set { l_id: var.n_id });
                }
                S::ExprStmt { expr } => {
                    self.reduce_expr(&expr, stmts, s)?;
                    // Drop the result to conserve stack height
                    stmts.push(Statement::Parametric {
                        param: Parametric::Drop,
                    });
                }
                S::ReturnStmt { expr, .. } => {
                    if let Some(e) = expr {
                        self.reduce_expr(&e, stmts, s)?;
                    }
                    stmts.push(Statement::Control {
                        cntrl: Control::Return,
                    })
                }
                S::WhileStmt { expr, block } => {
                    let block_id = s.fresh_bb_id();
                    let loop_id = s.fresh_bb_id();
                    let mut loop_stmts = Vec::new();

                    self.reduce_expr(&expr, &mut loop_stmts, s)?;
                    // If NOT expr, then jump to end of block
                    loop_stmts.push(Statement::Const { val: Value::I32(1) });
                    loop_stmts.push(Statement::Binop {
                        binop: Binop::I32Xor,
                    });
                    loop_stmts.push(Statement::Control {
                        cntrl: Control::BrIf(block_id),
                    });

                    self.reduce_block_rec(block, &mut loop_stmts, s)?;
                    loop_stmts.push(Statement::Control {
                        cntrl: Control::Br(loop_id),
                    });
                    let loop_block = Block::Loop {
                        id: loop_id,
                        stmts: loop_stmts,
                        t: None,
                    };
                    let block_block = Block::Block {
                        id: block_id,
                        stmts: vec![Statement::Block {
                            block: Box::new(loop_block),
                        }],
                        t: None,
                    };
                    stmts.push(Statement::Block {
                        block: Box::new(block_block),
                    });
                }
                S::IfStmt {
                    expr,
                    block,
                    else_block,
                } => {
                    self.reduce_expr(&expr, stmts, s)?;
                    let if_id = s.fresh_bb_id();
                    let mut then_stmts = Vec::new();
                    self.reduce_block_rec(block, &mut then_stmts, s)?;
                    let mut else_stmts = Vec::new();
                    if let Some(else_block) = else_block {
                        self.reduce_block_rec(else_block, &mut else_stmts, s)?;
                    }
                    let if_block = Block::If {
                        id: if_id,
                        then_stmts: then_stmts,
                        else_stmts: else_stmts,
                        t: None,
                    };
                    stmts.push(Statement::Block {
                        block: Box::new(if_block),
                    });
                }
            }
        }

        Ok(())
    }

    /// Push new statements that execute the given expression
    fn reduce_expr(
        &mut self,
        expression: &Expr,
        stmts: &mut Vec<Statement>,
        s: &mut State,
    ) -> Result<(), String> {
        match expression {
            Expr::Literal { value } => match value {
                V::Integer { val, t_id, .. } => {
                    let t = get_type(*t_id, s)?;
                    let val = match t {
                        Type::I32 => Value::I32((*val).try_into().unwrap()),
                        Type::I64 => Value::I64((*val).try_into().unwrap()),
                        _ => {
                            return Err(String::from("Integer constant of non integer type."));
                        }
                    };
                    stmts.push(Statement::Const { val: val })
                }
                V::Float { val, t_id, .. } => {
                    let t = get_type(*t_id, s)?;
                    let val = match t {
                        Type::F32 => Value::F32(*val as f32),
                        Type::F64 => Value::F64(*val),
                        _ => {
                            return Err(String::from("Float constant of non float type."));
                        }
                    };
                    stmts.push(Statement::Const { val: val })
                }
                V::Boolean { val, .. } => stmts.push(Statement::Const {
                    val: Value::I32(if *val { 1 } else { 0 }),
                }),
            },
            Expr::Variable { var } => stmts.push(Statement::Get { l_id: var.n_id }),
            Expr::Function { .. } => {
                return Err(String::from(
                    "Function as expression are not yet supported.",
                ))
            }
            Expr::Binary {
                expr_left,
                binop,
                expr_right,
                t_id: _,
                op_t_id,
                ..
            } => {
                let t = get_type(*op_t_id, s)?;
                let from_binop = get_binop(*binop, t)?;
                match from_binop {
                    FromBinop::Binop(binop) => {
                        self.reduce_expr(expr_left, stmts, s)?;
                        self.reduce_expr(expr_right, stmts, s)?;
                        stmts.push(Statement::Binop { binop: binop })
                    }
                    FromBinop::Relop(relop) => {
                        self.reduce_expr(expr_left, stmts, s)?;
                        self.reduce_expr(expr_right, stmts, s)?;
                        stmts.push(Statement::Relop { relop: relop })
                    }
                    FromBinop::Logical(logical) => match logical {
                        Logical::And => {
                            let if_id = s.fresh_bb_id();
                            let mut then_stmts = Vec::new();
                            self.reduce_expr(expr_right, &mut then_stmts, s)?;
                            let else_stmts = vec![Statement::Const { val: Value::I32(0) }];
                            let if_block = Block::If {
                                id: if_id,
                                then_stmts: then_stmts,
                                else_stmts: else_stmts,
                                t: Some(Type::I32),
                            };
                            self.reduce_expr(expr_left, stmts, s)?;
                            stmts.push(Statement::Block {
                                block: Box::new(if_block),
                            });
                        }
                        Logical::Or => {
                            let if_id = s.fresh_bb_id();
                            let then_stmts = vec![Statement::Const { val: Value::I32(1) }];
                            let mut else_stmts = Vec::new();
                            self.reduce_expr(expr_right, &mut else_stmts, s)?;
                            let if_block = Block::If {
                                id: if_id,
                                then_stmts: then_stmts,
                                else_stmts: else_stmts,
                                t: Some(Type::I32),
                            };
                            self.reduce_expr(expr_left, stmts, s)?;
                            stmts.push(Statement::Block {
                                block: Box::new(if_block),
                            });
                        }
                    },
                }
            }
            Expr::Unary {
                unop,
                expr,
                t_id,
                loc,
            } => {
                let t = get_type(*t_id, s)?;

                // corner cases:
                //  > (integer, minus): push zero first, then binary operator
                //  > (bool, not):      push one first, then binary operator
                match unop {
                    ASTUnop::Minus => match t {
                        Type::I32 => stmts.push(Statement::Const { val: Value::I32(0) }),
                        Type::I64 => stmts.push(Statement::Const { val: Value::I64(0) }),
                        _ => {}
                    },
                    ASTUnop::Not => {
                        match t {
                            // we should only have I32 for booleans if the typing phase is correct
                            Type::I32 => stmts.push(Statement::Const { val: Value::I32(1) }),
                            _ => self.err.report_internal(
                                *loc,
                                String::from("Not applied to something else than a boolean (I32) → error in type phase")
                            ),
                        }
                    }
                }
                // generic case: push evaluated value
                self.reduce_expr(expr, stmts, s)?;

                // generic case: push operator (might be unary or binary)
                let stmt = match unop {
                    ASTUnop::Minus => match t {
                        Type::I32 => Statement::Binop {
                            binop: Binop::I32Sub,
                        },
                        Type::I64 => Statement::Binop {
                            binop: Binop::I64Sub,
                        },
                        Type::F32 => Statement::Unop { unop: Unop::F32Neg },
                        Type::F64 => Statement::Unop { unop: Unop::F64Neg },
                    },
                    ASTUnop::Not => Statement::Binop {
                        binop: Binop::I32Xor,
                    },
                };
                stmts.push(stmt);
            }
            Expr::CallDirect { fun_id, args, .. } => {
                for arg in args {
                    self.reduce_expr(arg, stmts, s)?;
                }
                stmts.push(Statement::Call {
                    call: Call::Direct(*fun_id),
                })
            }
            Expr::CallIndirect { loc, .. } => self
                .err
                .report(*loc, String::from("Indirect call are not yet supported")),
        }
        Ok(())
    }

    fn reduce_asm_statements(
        &mut self,
        stmts: Vec<AsmStatement>,
        s: &mut State,
    ) -> Result<Vec<Statement>, String> {
        let mut reduced_stmts = Vec::with_capacity(stmts.len());
        for stmt in stmts {
            match self.reduce_asm_statement(stmt, s) {
                Ok(stmt) => reduced_stmts.push(stmt),
                Err(err) => self.err.report_no_loc(err), //TODO: track location
            }
        }
        Ok(reduced_stmts)
    }

    fn reduce_asm_statement(
        &mut self,
        stmt: AsmStatement,
        _s: &mut State,
    ) -> Result<Statement, String> {
        match stmt {
            AsmStatement::Const { val } => Ok(Statement::Const { val: val }),
            AsmStatement::Control { cntrl } => match cntrl {
                AsmControl::Return => Ok(Statement::Control {
                    cntrl: Control::Return,
                }),
            },
            AsmStatement::Parametric { param } => match param {
                AsmParametric::Drop => Ok(Statement::Parametric {
                    param: Parametric::Drop,
                }),
            },
        }
    }
}

fn get_binop(binop: ASTBinop, t: Type) -> Result<FromBinop, String> {
    match t {
        Type::I32 => match binop {
            ASTBinop::Plus => Ok(FromBinop::Binop(Binop::I32Add)),
            ASTBinop::Minus => Ok(FromBinop::Binop(Binop::I32Sub)),
            ASTBinop::Multiply => Ok(FromBinop::Binop(Binop::I32Mul)),
            ASTBinop::Divide => Ok(FromBinop::Binop(Binop::I32Div)),
            ASTBinop::Remainder => Ok(FromBinop::Binop(Binop::I32Rem)),

            ASTBinop::Equal => Ok(FromBinop::Relop(Relop::I32Eq)),
            ASTBinop::NotEqual => Ok(FromBinop::Relop(Relop::I32Ne)),
            ASTBinop::Less => Ok(FromBinop::Relop(Relop::I32Lt)),
            ASTBinop::Greater => Ok(FromBinop::Relop(Relop::I32Gt)),
            ASTBinop::LessEqual => Ok(FromBinop::Relop(Relop::I32Le)),
            ASTBinop::GreaterEqual => Ok(FromBinop::Relop(Relop::I32Ge)),

            ASTBinop::And => Ok(FromBinop::Logical(Logical::And)),
            ASTBinop::Or => Ok(FromBinop::Logical(Logical::Or)),

            _ => Err(String::from("Bad binary operator for i32")),
        },
        Type::I64 => match binop {
            ASTBinop::Plus => Ok(FromBinop::Binop(Binop::I64Add)),
            ASTBinop::Minus => Ok(FromBinop::Binop(Binop::I64Sub)),
            ASTBinop::Multiply => Ok(FromBinop::Binop(Binop::I64Mul)),
            ASTBinop::Divide => Ok(FromBinop::Binop(Binop::I64Div)),
            ASTBinop::Remainder => Ok(FromBinop::Binop(Binop::I64Rem)),

            ASTBinop::Equal => Ok(FromBinop::Relop(Relop::I64Eq)),
            ASTBinop::NotEqual => Ok(FromBinop::Relop(Relop::I64Ne)),
            ASTBinop::Less => Ok(FromBinop::Relop(Relop::I64Lt)),
            ASTBinop::Greater => Ok(FromBinop::Relop(Relop::I64Gt)),
            ASTBinop::LessEqual => Ok(FromBinop::Relop(Relop::I64Le)),
            ASTBinop::GreaterEqual => Ok(FromBinop::Relop(Relop::I64Ge)),

            _ => Err(String::from("Bad binary operator for i64")),
        },
        Type::F32 => match binop {
            ASTBinop::Plus => Ok(FromBinop::Binop(Binop::F32Add)),
            ASTBinop::Minus => Ok(FromBinop::Binop(Binop::F32Sub)),
            ASTBinop::Multiply => Ok(FromBinop::Binop(Binop::F32Mul)),
            ASTBinop::Divide => Ok(FromBinop::Binop(Binop::F32Div)),

            ASTBinop::Equal => Ok(FromBinop::Relop(Relop::F32Eq)),
            ASTBinop::NotEqual => Ok(FromBinop::Relop(Relop::F32Ne)),
            ASTBinop::Less => Ok(FromBinop::Relop(Relop::F32Lt)),
            ASTBinop::Greater => Ok(FromBinop::Relop(Relop::F32Gt)),
            ASTBinop::LessEqual => Ok(FromBinop::Relop(Relop::F32Le)),
            ASTBinop::GreaterEqual => Ok(FromBinop::Relop(Relop::F32Ge)),

            _ => Err(String::from("Bad binary operator for f32")),
        },
        Type::F64 => match binop {
            ASTBinop::Plus => Ok(FromBinop::Binop(Binop::F64Add)),
            ASTBinop::Minus => Ok(FromBinop::Binop(Binop::F64Sub)),
            ASTBinop::Multiply => Ok(FromBinop::Binop(Binop::F64Mul)),
            ASTBinop::Divide => Ok(FromBinop::Binop(Binop::F64Div)),

            ASTBinop::Equal => Ok(FromBinop::Relop(Relop::F64Eq)),
            ASTBinop::NotEqual => Ok(FromBinop::Relop(Relop::F64Ne)),
            ASTBinop::Less => Ok(FromBinop::Relop(Relop::F64Lt)),
            ASTBinop::Greater => Ok(FromBinop::Relop(Relop::F64Gt)),
            ASTBinop::LessEqual => Ok(FromBinop::Relop(Relop::F64Le)),
            ASTBinop::GreaterEqual => Ok(FromBinop::Relop(Relop::F64Ge)),

            _ => Err(String::from("Bad binary operator for f64")),
        },
    }
}

fn get_type(t_id: TypeId, s: &State) -> Result<Type, String> {
    let t = s.types.get(t_id);
    match t {
        ASTTypes::Any | ASTTypes::Bug | ASTTypes::Unit => Err(format!(
            "Invalid type in MIR generation: {} for t_id: {}",
            t, t_id
        )),
        ASTTypes::I32 => Ok(Type::I32),
        ASTTypes::I64 => Ok(Type::I64),
        ASTTypes::F32 => Ok(Type::F32),
        ASTTypes::F64 => Ok(Type::F64),
        ASTTypes::Bool => Ok(Type::I32),
        ASTTypes::Fun(_, _) => Err(String::from("Function as a value are not yet implemented")),
    }
}

fn convert_type(t: &ASTTypes) -> Result<Type, String> {
    match t {
        ASTTypes::Any | ASTTypes::Bug | ASTTypes::Unit => {
            Err(format!("Invalid type in MIR generation: {}", t))
        }
        ASTTypes::I32 => Ok(Type::I32),
        ASTTypes::I64 => Ok(Type::I64),
        ASTTypes::F32 => Ok(Type::F32),
        ASTTypes::F64 => Ok(Type::F64),
        ASTTypes::Bool => Ok(Type::I32),
        ASTTypes::Fun(_, _) => Err(String::from("Function as a value are not yet implemented")),
    }
}
