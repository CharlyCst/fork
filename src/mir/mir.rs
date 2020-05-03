use std::fmt;

pub struct Program {
    pub funs: Vec<Function>,
}

pub struct Function {
    pub ident: String,
    pub params: Vec<LocalId>,
    pub param_types: Vec<Type>,
    pub ret_types: Vec<Type>,
    pub locals: Vec<Local>,
    pub body: Block,
    pub exported: bool,
}

pub type LocalId = usize; // For now NameId are used as LocalId

pub struct Local {
    pub id: LocalId,
    pub t: Type,
}

pub type BasicBlockId = usize;

pub enum Block {
    Block {
        id: BasicBlockId,
        stmts: Vec<Statement>,
    },
    Loop {
        id: BasicBlockId,
        stmts: Vec<Statement>,
    },
    If {
        id: BasicBlockId,
        then_stmts: Vec<Statement>,
        else_stmts: Vec<Statement>,
    },
}

pub enum Statement {
    Set { l_id: LocalId },
    Get { l_id: LocalId },
    Const { val: Value },
    Block { block: Box<Block> },
    Unop { unop: Unop },
    Binop { binop: Binop },
    Relop { relop: Relop },
    Control { cntrl: Control },
    Parametric { param: Parametric },
}

pub enum Control {
    Return,
    Br(BasicBlockId),
    BrIf(BasicBlockId),
}

pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

pub enum Unop {
    I32Neg,
    I64Neg,
    F32Neg,
    F64Neg,
}

pub enum Binop {
    I32Xor,
    I32Add,
    I32Sub,
    I32Mul,
    I32Div,
    I32Rem,

    I64Add,
    I64Sub,
    I64Mul,
    I64Div,
    I64Rem,

    F32Add,
    F32Sub,
    F32Mul,
    F32Div,

    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
}

pub enum Relop {
    I32Eq,
    I32Ne,
    I32Lt,
    I32Gt,
    I32Le,
    I32Ge,

    I64Eq,
    I64Ne,
    I64Lt,
    I64Gt,
    I64Le,
    I64Ge,

    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,

    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,
}

pub enum Parametric {
    Drop,
}

#[derive(Copy, Clone)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let funs = self
            .funs
            .iter()
            .map(|fun| format!("{}", fun))
            .collect::<Vec<String>>()
            .join("\n\n");
        write!(f, "MIR {{\n{}\n}}", funs)
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .param_types
            .iter()
            .map(|t| format!("{}", t))
            .collect::<Vec<String>>()
            .join(", ");
        let ret = self
            .ret_types
            .iter()
            .map(|t| format!("{}", t))
            .collect::<Vec<String>>()
            .join(", ");
        let locals = self
            .locals
            .iter()
            .map(|l| format!("{}", l))
            .collect::<Vec<String>>()
            .join("");
        let mut body = Vec::new();
        for line in format!("{}", self.body).split("\n") {
            let mut indented_line = String::from("    ");
            indented_line.push_str(line);
            body.push(indented_line)
        }
        write!(
            f,
            "  {}({}) {} {{\n{}{}\n  }}",
            self.ident,
            params,
            ret,
            locals,
            body.iter().map(|s| &**s).collect::<Vec<&str>>().join("\n")
        )
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut strs = Vec::new();
        let stmts = match self {
            Block::Block { id, stmts } => {
                strs.push(format!("block {} {{", id));
                stmts
            }
            Block::Loop { id, stmts } => {
                strs.push(format!("loop {} {{", id));
                stmts
            }
            Block::If { id, then_stmts, .. } => {
                strs.push(format!("if {} {{", id));
                then_stmts
            }
        };
        for stmt in stmts.iter() {
            for line in format!("{}", stmt).split("\n") {
                let mut indented_line = String::from("  ");
                indented_line.push_str(line);
                strs.push(indented_line)
            }
        }
        if let Block::If { else_stmts, .. } = self {
            strs.push(String::from("} else {"));
            for stmt in else_stmts.iter() {
                for line in format!("{}", stmt).split("\n") {
                    let mut indented_line = String::from("  ");
                    indented_line.push_str(line);
                    strs.push(indented_line)
                }
            }
        };
        strs.push(String::from("}"));
        write!(
            f,
            "{}",
            strs.iter().map(|s| &**s).collect::<Vec<&str>>().join("\n"),
        )
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::Get { l_id } => write!(f, "get _{}", l_id),
            Statement::Set { l_id } => write!(f, "set _{}", l_id),
            Statement::Unop { unop } => write!(f, "{}", unop),
            Statement::Binop { binop } => write!(f, "{}", binop),
            Statement::Relop { relop } => write!(f, "{}", relop),
            Statement::Parametric { param } => write!(f, "{}", param),
            Statement::Block { block } => write!(f, "{}", block),
            Statement::Control { cntrl } => write!(f, "{}", cntrl),
            Statement::Const { val } => match val {
                Value::I32(x) => write!(f, "i32 {}", x),
                Value::I64(x) => write!(f, "i64 {}", x),
                Value::F32(x) => write!(f, "f32 {}", x),
                Value::F64(x) => write!(f, "f64 {}", x),
            },
        }
    }
}

impl fmt::Display for Unop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unop::I32Neg => write!(f, "i32.ne"),
            Unop::I64Neg => write!(f, "i64.ne"),
            Unop::F32Neg => write!(f, "f32.ne"),
            Unop::F64Neg => write!(f, "f64.ne"),
        }
    }
}

impl fmt::Display for Binop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Binop::I32Xor => write!(f, "i32.xor"),
            Binop::I32Add => write!(f, "i32.add"),
            Binop::I32Sub => write!(f, "i32.sub"),
            Binop::I32Mul => write!(f, "i32.mul"),
            Binop::I32Div => write!(f, "i32.div"),
            Binop::I32Rem => write!(f, "i32.rem"),

            Binop::I64Add => write!(f, "i64.add"),
            Binop::I64Sub => write!(f, "i64.sub"),
            Binop::I64Mul => write!(f, "i64.mul"),
            Binop::I64Div => write!(f, "i64.div"),
            Binop::I64Rem => write!(f, "i64.rem"),

            Binop::F32Add => write!(f, "f32.add"),
            Binop::F32Sub => write!(f, "f32.sub"),
            Binop::F32Mul => write!(f, "f32.mul"),
            Binop::F32Div => write!(f, "f32.div"),

            Binop::F64Add => write!(f, "f64.add"),
            Binop::F64Sub => write!(f, "f64.sub"),
            Binop::F64Mul => write!(f, "f64.mul"),
            Binop::F64Div => write!(f, "f64.div"),
        }
    }
}

impl fmt::Display for Relop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Relop::I32Eq => write!(f, "i32.eq"),
            Relop::I32Ne => write!(f, "i32.ne"),
            Relop::I32Lt => write!(f, "i32.lt"),
            Relop::I32Gt => write!(f, "i32.gt"),
            Relop::I32Le => write!(f, "i32.le"),
            Relop::I32Ge => write!(f, "i32.ge"),

            Relop::I64Eq => write!(f, "i64.eq"),
            Relop::I64Ne => write!(f, "i64.ne"),
            Relop::I64Lt => write!(f, "i64.lt"),
            Relop::I64Gt => write!(f, "i64.gt"),
            Relop::I64Le => write!(f, "i64.le"),
            Relop::I64Ge => write!(f, "i64.ge"),

            Relop::F32Eq => write!(f, "f32.eq"),
            Relop::F32Ne => write!(f, "f32.ne"),
            Relop::F32Lt => write!(f, "f32.lt"),
            Relop::F32Gt => write!(f, "f32.gt"),
            Relop::F32Le => write!(f, "f32.le"),
            Relop::F32Ge => write!(f, "f32.ge"),

            Relop::F64Eq => write!(f, "f64.eq"),
            Relop::F64Ne => write!(f, "f64.ne"),
            Relop::F64Lt => write!(f, "f64.lt"),
            Relop::F64Gt => write!(f, "f64.gt"),
            Relop::F64Le => write!(f, "f64.le"),
            Relop::F64Ge => write!(f, "f64.ge"),
        }
    }
}

impl fmt::Display for Parametric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Parametric::Drop => write!(f, "drop"),
        }
    }
}

impl fmt::Display for Control {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Control::Return => write!(f, "return"),
            Control::Br(bb_id) => write!(f, "br {}", bb_id),
            Control::BrIf(bb_id) => write!(f, "br_if {}", bb_id),
        }
    }
}

impl fmt::Display for Local {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "    _{}\n", self.id)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
        }
    }
}