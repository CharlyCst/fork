use crate::parse;

use self::names::NameStore;
use self::types::{ConstraintStore, TypeStore, TypeVarStore};

pub use self::names::NameId;
pub use self::types::TypeId;

mod names;
mod resolver;
mod type_check;
mod types;

pub struct Program {
    pub funs: Vec<parse::Function>,
    pub names: NameStore,
    pub types: TypeVarStore,
    pub constraints: ConstraintStore,
}

pub struct TypedProgram {
    pub funs: Vec<parse::Function>,
    pub names: NameStore,
    pub types: TypeStore,
}

pub fn to_mir(functions: Vec<parse::Function>) {
    let mut name_resolver = resolver::NameResolver::new();
    let program = name_resolver.resolve(functions);

    println!("\n/// Name Resolution ///\n");

    println!("{}\n", program.names);
    println!("{}\n", program.types);
    println!("{}\n", program.constraints);

    println!("\n/// Type Checking ///\n");

    let mut type_checker = type_check::TypeChecker::new();
    type_checker.check(program);
}
