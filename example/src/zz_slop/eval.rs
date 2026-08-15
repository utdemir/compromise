use std::collections::HashMap;

use crate::eval::EvalOutput;
use crate::parser::{Expr, Stmt};

#[derive(Clone)]
enum Value {
    Num(i64),
    Closure(String, OwnedExpr, HashMap<String, Value>),
}

#[derive(Clone)]
enum OwnedExpr {
    Num(i64),
    Var(String),
    Lam(String, Box<OwnedExpr>),
    App(Box<OwnedExpr>, Box<OwnedExpr>),
    Add(Box<OwnedExpr>, Box<OwnedExpr>),
    Sub(Box<OwnedExpr>, Box<OwnedExpr>),
    Mul(Box<OwnedExpr>, Box<OwnedExpr>),
    Div(Box<OwnedExpr>, Box<OwnedExpr>),
}

impl From<&Expr> for OwnedExpr {
    fn from(expression: &Expr) -> Self {
        match expression {
            Expr::Num(value) => Self::Num(*value),
            Expr::Var(name) => Self::Var(name.clone()),
            Expr::Lam(name, body) => Self::Lam(name.clone(), Box::new(Self::from(&**body))),
            Expr::App(left, right) => Self::App(
                Box::new(Self::from(&**left)),
                Box::new(Self::from(&**right)),
            ),
            Expr::Add(left, right) => Self::Add(
                Box::new(Self::from(&**left)),
                Box::new(Self::from(&**right)),
            ),
            Expr::Sub(left, right) => Self::Sub(
                Box::new(Self::from(&**left)),
                Box::new(Self::from(&**right)),
            ),
            Expr::Mul(left, right) => Self::Mul(
                Box::new(Self::from(&**left)),
                Box::new(Self::from(&**right)),
            ),
            Expr::Div(left, right) => Self::Div(
                Box::new(Self::from(&**left)),
                Box::new(Self::from(&**right)),
            ),
        }
    }
}

pub struct Eval {
    variables: HashMap<String, Value>,
}

pub fn new() -> Eval {
    Eval {
        variables: HashMap::new(),
    }
}

pub fn eval_str(eval: &mut Eval, input: &str) -> Result<EvalOutput, &'static str> {
    let mut result = EvalOutput::Dec;
    let mut saw_statement = false;

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        saw_statement = true;
        match crate::zz_slop::parser::parse(line)? {
            Stmt::Let(name, expression) => {
                let value = eval_expr(&eval.variables, &expression)?;
                eval.variables.insert(name, value);
                result = EvalOutput::Dec;
            }
            Stmt::Eval(expression) => {
                result = match eval_expr(&eval.variables, &expression)? {
                    Value::Num(value) => EvalOutput::Val(value),
                    Value::Closure(..) => EvalOutput::Lam,
                };
            }
        }
    }

    if saw_statement {
        Ok(result)
    } else {
        Err("syntax error")
    }
}

fn eval_expr(variables: &HashMap<String, Value>, expression: &Expr) -> Result<Value, &'static str> {
    eval_owned(variables, &OwnedExpr::from(expression))
}

fn eval_owned(
    variables: &HashMap<String, Value>,
    expression: &OwnedExpr,
) -> Result<Value, &'static str> {
    match expression {
        OwnedExpr::Num(value) => Ok(Value::Num(*value)),
        OwnedExpr::Var(name) => variables.get(name).cloned().ok_or("unbound variable"),
        OwnedExpr::Lam(parameter, body) => Ok(Value::Closure(
            parameter.clone(),
            body.as_ref().clone(),
            variables.clone(),
        )),
        OwnedExpr::App(function, argument) => {
            let Value::Closure(parameter, body, mut environment) = eval_owned(variables, function)?
            else {
                return Err("not a function");
            };
            let argument = eval_owned(variables, argument)?;
            environment.insert(parameter, argument);
            eval_owned(&environment, &body)
        }
        OwnedExpr::Add(left, right) => arithmetic(variables, left, right, i64::checked_add),
        OwnedExpr::Sub(left, right) => arithmetic(variables, left, right, i64::checked_sub),
        OwnedExpr::Mul(left, right) => arithmetic(variables, left, right, i64::checked_mul),
        OwnedExpr::Div(left, right) => {
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
    left: &OwnedExpr,
    right: &OwnedExpr,
) -> Result<(i64, i64), &'static str> {
    match (eval_owned(variables, left)?, eval_owned(variables, right)?) {
        (Value::Num(left), Value::Num(right)) => Ok((left, right)),
        _ => Err("not a number"),
    }
}

fn arithmetic(
    variables: &HashMap<String, Value>,
    left: &OwnedExpr,
    right: &OwnedExpr,
    operation: fn(i64, i64) -> Option<i64>,
) -> Result<Value, &'static str> {
    let (left, right) = numbers(variables, left, right)?;
    operation(left, right).map(Value::Num).ok_or("overflow")
}
