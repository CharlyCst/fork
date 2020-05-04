use super::types::id::T_ID_INTEGER;
use super::types::{Type, TypeConstraint, TypeStore, TypeVarStore};
use super::{ResolvedProgram, TypedProgram};
use crate::error::ErrorHandler;

use std::cmp::Ordering;

pub struct TypeChecker {
    error_handler: ErrorHandler,
}

impl TypeChecker {
    pub fn new() -> TypeChecker {
        TypeChecker {
            error_handler: ErrorHandler::new(),
        }
    }

    pub fn check(&mut self, prog: ResolvedProgram) -> TypedProgram {
        let mut type_vars = prog.types;
        let constraints = prog.constraints;

        let mut making_progress = true;
        while making_progress {
            making_progress = false;
            for constr in &constraints {
                let progress = self.apply_constr(constr, &mut type_vars);
                making_progress = progress || making_progress;

                // if progress { // May be useful for debugging, so I let that here for now ¯\_(ツ)_/¯
                //     match constr {
                //         TypeConstraint::Equality(t_1, t_2) => println!("{:>4} = {:>4}", t_1, t_2),
                //         TypeConstraint::Included(t_1, t_2) => print!("{:>4} ⊂ {:>4}", t_1, t_2),
                //         TypeConstraint::Return(fun_t, ret_t) => {
                //             print!("{:>4} -> {:>3}", fun_t, ret_t)
                //         }
                //     };
                // }
            }
        }

        let store = self.build_store(&type_vars);
        TypedProgram {
            funs: prog.funs,
            names: prog.names,
            types: store,
        }
    }

    fn build_store(&mut self, var_store: &TypeVarStore) -> TypeStore {
        let integers = var_store.get(T_ID_INTEGER);
        let mut store = TypeStore::new();
        for var in var_store {
            if var.types.len() == 1 {
                store.put(var.types[0].clone())
            } else if var.types.len() == 0 {
                // TODO: improve error handling...
                self.error_handler
                    .report_line(0, "Could not find a type satisfying constraint")
            } else {
                // Choose arbitrary type if applicable
                if var.types == integers.types {
                    store.put(Type::I64);
                } else {
                    // TODO: improve error handling...
                    self.error_handler.report_line(0, "Could not infer type")
                }
            }
        }

        store
    }

    // fn try_reduce(types: Vec<Type>) -> Result<Type, String>{
    //     if types.len() == 1 {}
    // }

    // Apply a constraint, return true if the constraint helped removing type candidates,
    // i.e. we are making progress
    fn apply_constr(&mut self, constr: &TypeConstraint, store: &mut TypeVarStore) -> bool {
        match constr {
            TypeConstraint::Equality(t_id_1, t_id_2) => {
                self.constr_equality(*t_id_1, *t_id_2, store)
            }
            TypeConstraint::Included(t_id_1, t_id_2) => {
                self.constr_included(*t_id_1, *t_id_2, store)
            }
            TypeConstraint::Return(t_id_fun, t_id) => self.constr_return(*t_id_fun, *t_id, store),
        }
    }

    fn constr_equality(&mut self, t_id_1: usize, t_id_2: usize, store: &mut TypeVarStore) -> bool {
        let t_1 = &store.get(t_id_1).types;
        let t_2 = &store.get(t_id_2).types;

        // Special cases
        if t_1.len() > 0 && t_1[0] == Type::Any {
            if t_2.len() > 0 && t_2[0] == Type::Any {
                return false;
            }
            let t = t_2.clone();
            store.replace(t_id_1, t);
            return true;
        } else if t_2.len() > 0 && t_2[0] == Type::Any {
            let t = t_1.clone();
            store.replace(t_id_2, t);
            return true;
        }

        // Can not infer types
        if t_1.len() == 0 || t_2.len() == 0 {
            let loc = store.get(t_id_1).loc;
            self.error_handler
                .report(loc, "Could not infer a type satisfying constraints");
            return false;
        }

        let mut t = Vec::new();
        let mut idx_1 = 0;
        let mut idx_2 = 0;
        let mut progress = false || t_1.len() != t_2.len();
        while idx_1 < t_1.len() && idx_2 < t_2.len() {
            match t_1[idx_1].cmp(&t_2[idx_2]) {
                Ordering::Less => {
                    idx_1 += 1;
                    progress = true;
                }
                Ordering::Greater => {
                    idx_2 += 1;
                    progress = true;
                }
                Ordering::Equal => {
                    t.push(t_1[idx_1].clone());
                    idx_1 += 1;
                    idx_2 += 1;
                }
            }
        }

        store.replace(t_id_1, t.clone());
        store.replace(t_id_2, t);
        progress
    }

    fn constr_included(&mut self, t_id_1: usize, t_id_2: usize, store: &mut TypeVarStore) -> bool {
        let t_1 = &store.get(t_id_1).types;
        let t_2 = &store.get(t_id_2).types;

        // Special case
        if t_1.len() > 0 && t_1[0] == Type::Any {
            if t_2.len() > 0 && t_2[0] == Type::Any {
                return false;
            }
            let t = t_2.clone();
            store.replace(t_id_1, t);
            return true;
        }

        let mut t = Vec::new();
        let mut idx_1 = 0;
        let mut idx_2 = 0;
        let mut progress = false;
        while idx_1 < t_1.len() && idx_2 < t_2.len() {
            match t_1[idx_1].cmp(&t_2[idx_2]) {
                Ordering::Less => {
                    idx_1 += 1;
                    progress = true;
                }
                Ordering::Greater => {
                    idx_2 += 1;
                }
                Ordering::Equal => {
                    t.push(t_1[idx_1].clone());
                    idx_1 += 1;
                    idx_2 += 1;
                }
            }
        }

        store.replace(t_id_1, t);
        progress
    }

    fn constr_return(&mut self, t_id_fun: usize, t_id: usize, store: &mut TypeVarStore) -> bool {
        let t_fun = store.get(t_id_fun);
        let ts = store.get(t_id);

        if t_fun.types.len() != 1 {
            self.error_handler
                .report_internal_loc(t_fun.loc, "Return type constraint with ambiguous fun type");
            return false;
        }

        let ret_t = match &t_fun.types[0] {
            Type::Fun(_, ret_t) => ret_t,
            _ => {
                self.error_handler.report_internal_loc(
                    t_fun.loc,
                    "Return type constraint used on a non function type",
                );
                return false;
            }
        };

        if ret_t.len() != 1 {
            self.error_handler.report_internal_loc(
                t_fun.loc,
                "Function returning multiple values are not yet supported",
            );
            return false;
        }

        let ret_t = &ret_t[0];
        for t in &ts.types {
            if t == ret_t {
                let progress = ts.types.len() > 1;
                let typ = vec![t.clone()];
                store.replace(t_id, typ);
                return progress;
            }
        }

        self.error_handler
            .report(ts.loc, "Return value has wrong type");
        return false;
    }
}
