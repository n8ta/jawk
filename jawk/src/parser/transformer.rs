use crate::parser::{PatternAction, Stmt, TypedExpr};
use crate::{Expr};

fn group_vec_of_stmt(mut stmts: Vec<Stmt>) -> Stmt {
    if stmts.len() == 1 {
        stmts.pop().unwrap()
    } else {
        Stmt::Group(stmts)
    }
}

// Turn a program into just a single Stmt
pub fn transform(begins: Vec<Stmt>, ends: Vec<Stmt>, pas: Vec<PatternAction>) -> Stmt {
    let mut prog = begins;
    let mut every_line_stms = vec![];
    for pattern in pas {
        let stmt = if let Some(test) = pattern.pattern {
            Stmt::If(test, Box::new(pattern.action), None)
        } else {
            pattern.action
        };
        every_line_stms.push(stmt)
    }
    if every_line_stms.len() > 0 {
        let line_loop = Stmt::While(
            TypedExpr::new(Expr::NextLine),
            Box::new(group_vec_of_stmt(every_line_stms)),
        );
        prog.push(line_loop);
    }

    for end in ends {
        prog.push(end);
    }
    group_vec_of_stmt(prog)
}