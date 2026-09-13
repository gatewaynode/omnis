//! Static checks over a compiled formula: every variable is a declared input, every call is an
//! allowed function, and no string or character literal appears. Positions name the offending
//! node so a pack error reads `file:line:column`.

use crate::engine::ALLOWED_CALLS;
use crate::error::CompileError;
use rhai::{AST, ASTNode, Expr, Position};

fn at(position: Position, message: String) -> CompileError {
    CompileError::at(position.line(), position.position(), message)
}

/// Walk `ast` and return the first violation.
pub(crate) fn check(ast: &AST, inputs: &[String]) -> Result<(), CompileError> {
    let mut found: Option<CompileError> = None;
    ast.walk(&mut |path: &[ASTNode]| {
        let Some(ASTNode::Expr(expr)) = path.last() else {
            return true;
        };
        let violation = match expr {
            Expr::Variable(var, _, pos) => {
                let name: &str = &var.1;
                (!inputs.iter().any(|i| i == name))
                    .then(|| at(*pos, format!("unknown input '{name}'")))
            }
            Expr::FnCall(call, pos) => {
                let name: &str = &call.name;
                (call.op_token.is_none() && !ALLOWED_CALLS.contains(&name))
                    .then(|| at(*pos, format!("unknown function '{name}'")))
            }
            Expr::StringConstant(_, pos) | Expr::InterpolatedString(_, pos) => {
                Some(at(*pos, "strings are not allowed in formulas".to_owned()))
            }
            Expr::CharConstant(_, pos) => Some(at(
                *pos,
                "characters are not allowed in formulas".to_owned(),
            )),
            _ => None,
        };
        match violation {
            Some(error) => {
                found = Some(error);
                false
            }
            None => true,
        }
    });
    found.map_or(Ok(()), Err)
}
