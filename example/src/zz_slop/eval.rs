use std::collections::HashMap;

use crate::eval::EvalOutput;
use crate::parser::{Expr, Stmt};

#[derive(Clone)]
enum Value {
    Num(i64),
    Closure(String, Expr, HashMap<String, Value>),
}

pub struct Eval {
    variables: HashMap<String, Value>,
}

pub fn new() -> Eval {
    Eval {
        variables: HashMap::new(),
    }
}

pub fn eval(eval: &mut Eval, input: &Stmt) -> Result<EvalOutput, &'static str> {
    match input {
        Stmt::Let(name, expression) => {
            let value = eval_expr(&eval.variables, expression)?;
            eval.variables.insert(name.clone(), value);
            Ok(EvalOutput::Dec)
        }
        Stmt::Eval(expression) => match eval_expr(&eval.variables, expression)? {
            Value::Num(value) => Ok(EvalOutput::Val(value)),
            Value::Closure(..) => Ok(EvalOutput::Lam),
        },
    }
}

fn eval_expr(variables: &HashMap<String, Value>, expression: &Expr) -> Result<Value, &'static str> {
    match expression {
        Expr::Num(value) => Ok(Value::Num(*value)),
        Expr::Var(name) => variables.get(name).cloned().ok_or("unbound variable"),
        Expr::Lam(parameter, body) => Ok(Value::Closure(
            parameter.clone(),
            body.as_ref().clone(),
            variables.clone(),
        )),
        Expr::App(function, argument) => {
            let Value::Closure(parameter, body, mut environment) = eval_expr(variables, function)?
            else {
                return Err("not a function");
            };
            let argument = eval_expr(variables, argument)?;
            environment.insert(parameter, argument);
            eval_expr(&environment, &body)
        }
        Expr::Add(left, right) => arithmetic(variables, left, right, i64::checked_add),
        Expr::Sub(left, right) => arithmetic(variables, left, right, i64::checked_sub),
        Expr::Mul(left, right) => arithmetic(variables, left, right, i64::checked_mul),
        Expr::Div(left, right) => {
            let (dividend, divisor) = numbers(variables, left, right)?;
            if divisor == 0 {
                return Err("division by zero");
            }
            dividend
                .checked_div(divisor)
                .map(Value::Num)
                .ok_or("overflow")
        }
    }
}

fn numbers(
    variables: &HashMap<String, Value>,
    left: &Expr,
    right: &Expr,
) -> Result<(i64, i64), &'static str> {
    match (eval_expr(variables, left)?, eval_expr(variables, right)?) {
        (Value::Num(left), Value::Num(right)) => Ok((left, right)),
        _ => Err("not a number"),
    }
}

fn arithmetic(
    variables: &HashMap<String, Value>,
    left: &Expr,
    right: &Expr,
    operation: fn(i64, i64) -> Option<i64>,
) -> Result<Value, &'static str> {
    let (left, right) = numbers(variables, left, right)?;
    operation(left, right).map(Value::Num).ok_or("overflow")
}
