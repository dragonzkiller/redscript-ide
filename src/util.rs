use redscript::ast::{Expr, Ident, Pos, TypeName};
use redscript::bundle::{ConstantPool, PoolIndex};
use redscript::definition::Function;
use redscript_compiler::typechecker::TypedAst;

use crate::error::Error;

pub fn find_in_seq<'a>(haystack: &'a [Expr<TypedAst>], needle: Pos) -> Option<&'a Expr<TypedAst>> {
    // TODO: use binary search
    haystack.iter().find_map(|expr| find_in_expr(expr, needle))
}

pub fn find_in_expr<'a>(haystack: &'a Expr<TypedAst>, needle: Pos) -> Option<&'a Expr<TypedAst>> {
    if haystack.span().contains(needle) {
        let res = match haystack {
            Expr::ArrayLit(exprs, _, _) => find_in_seq(exprs, needle),
            Expr::Declare(_, _, Some(expr), _) => find_in_expr(expr, needle),
            Expr::Cast(_, expr, _) => find_in_expr(expr, needle),
            Expr::Assign(_, expr, _) => find_in_expr(expr, needle),
            Expr::Call(_, _, args, _) => find_in_seq(args, needle),
            Expr::MethodCall(expr, _, args, _) => find_in_expr(expr, needle).or_else(|| find_in_seq(args, needle)),
            Expr::Member(expr, _, _) => find_in_expr(expr, needle),
            Expr::ArrayElem(expr, index, _) => find_in_expr(expr, needle).or_else(|| find_in_expr(index, needle)),
            Expr::New(_, args, _) => find_in_seq(args, needle),
            Expr::Return(Some(expr), _) => find_in_expr(expr, needle),
            Expr::Seq(seq) => seq.exprs.iter().find_map(|expr| find_in_expr(expr, needle)),
            Expr::Switch(expr, cases, Some(default), _) => find_in_expr(expr, needle)
                .or_else(|| {
                    cases.iter().find_map(|case| {
                        find_in_expr(&case.matcher, needle).or_else(|| find_in_seq(&case.body.exprs, needle))
                    })
                })
                .or_else(|| find_in_seq(&default.exprs, needle)),
            Expr::Switch(expr, cases, None, _) => find_in_expr(expr, needle).or_else(|| {
                cases.iter().find_map(|case| {
                    find_in_expr(&case.matcher, needle).or_else(|| find_in_seq(&case.body.exprs, needle))
                })
            }),
            Expr::If(expr, if_, Some(else_), _) => find_in_expr(expr, needle)
                .or_else(|| find_in_seq(&if_.exprs, needle))
                .or_else(|| find_in_seq(&else_.exprs, needle)),
            Expr::If(expr, if_, None, _) => find_in_expr(expr, needle).or_else(|| find_in_seq(&if_.exprs, needle)),
            Expr::Conditional(expr, true_, false_, _) => find_in_expr(expr, needle)
                .or_else(|| find_in_expr(true_, needle))
                .or_else(|| find_in_expr(false_, needle)),
            Expr::While(expr, seq, _) => {
                find_in_expr(expr, needle).or_else(|| seq.exprs.iter().find_map(|arg| find_in_expr(arg, needle)))
            }
            Expr::ForIn(_, expr, seq, _) => {
                find_in_expr(expr, needle).or_else(|| seq.exprs.iter().find_map(|arg| find_in_expr(arg, needle)))
            }
            Expr::BinOp(lhs, rhs, _, _) => find_in_expr(lhs, needle).or_else(|| find_in_expr(rhs, needle)),
            Expr::UnOp(expr, _, _) => find_in_expr(expr, needle),
            _ => Some(haystack),
        };
        res.or_else(|| Some(haystack))
    } else {
        None
    }
}

pub fn render_function(idx: PoolIndex<Function>, short: bool, pool: &ConstantPool) -> Result<String, Error> {
    let name = pool.def_name(idx)?;
    let pretty_name = if short {
        ""
    } else {
        name.split(";").next().unwrap_or(&name)
    };
    let fun = pool.function(idx)?;

    let mut args = String::new();
    for (i, param_idx) in fun.parameters.iter().enumerate() {
        let name = pool.def_name(*param_idx)?;
        let param = pool.parameter(*param_idx)?;
        let type_name = TypeName::from_repr(&pool.def_name(param.type_)?);
        if i != 0 {
            args.push_str(", ");
        }
        args.push_str(&format!("{}: {}", name, type_name.pretty()));
    }

    let ret_type = if let Some(ret_idx) = fun.return_type {
        TypeName::from_repr(&pool.def_name(ret_idx)?).pretty()
    } else {
        Ident::Static("Void")
    };

    Ok(format!("{}({}) -> {}", pretty_name, args, ret_type))
}
