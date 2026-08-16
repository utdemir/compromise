use compromise::slop;

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Num(i64),
    Var(String),
    Lam(String, Box<Expr>),
    App(Box<Expr>, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    Let(String, Expr),
    Eval(Expr),
}

#[slop]
pub fn parse(input: &str) -> Result<Stmt, &'static str>;

#[cfg(test)]

mod tests {
    use super::*;

    macro_rules! case {
        ($name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $name() {
                use Expr::*;
                use Stmt::*;
                let result = parse($input).unwrap();
                assert_eq!(result, $expected);
            }
        };
    }

    case!(test_num, "42", Eval(Num(42)));
    case!(test_var, "x", Eval(Var("x".to_string())));
    case!(
        test_add,
        "1 + 2",
        Eval(Add(Box::new(Num(1)), Box::new(Num(2))))
    );
    case!(
        test_sub,
        "3 - 4",
        Eval(Sub(Box::new(Num(3)), Box::new(Num(4))))
    );
    case!(
        test_div,
        "10 / 2",
        Eval(Div(Box::new(Num(10)), Box::new(Num(2))))
    );
    case!(
        test_lam1,
        "|x| x + 1",
        Eval(Lam(
            "x".to_string(),
            Box::new(Add(Box::new(Var("x".to_string())), Box::new(Num(1))))
        ))
    );
    case!(
        test_lam2,
        "|x, y| x + y",
        Eval(Lam(
            "x".to_string(),
            Box::new(Lam(
                "y".to_string(),
                Box::new(Add(
                    Box::new(Var("x".to_string())),
                    Box::new(Var("y".to_string()))
                ))
            ))
        ))
    );
    case!(
        test_app,
        "(|x| x + 1)(5)",
        Eval(App(
            Box::new(Lam(
                "x".to_string(),
                Box::new(Add(Box::new(Var("x".to_string())), Box::new(Num(1))))
            )),
            Box::new(Num(5))
        ))
    );
    case!(test_stmt, "x = 10", Let("x".to_string(), Num(10)));

    case!(
        test_precedence,
        "1 + 2 * 3",
        Eval(Add(
            Box::new(Num(1)),
            Box::new(Mul(Box::new(Num(2)), Box::new(Num(3))))
        ))
    );
    case!(
        test_parentheses,
        "(1 + 2) * 3",
        Eval(Mul(
            Box::new(Add(Box::new(Num(1)), Box::new(Num(2)))),
            Box::new(Num(3))
        ))
    );
    case!(
        test_assignment,
        "x = 5 * 3",
        Let("x".to_string(), Mul(Box::new(Num(5)), Box::new(Num(3))))
    );

    macro_rules! case_eq {
        ($name:ident, $lhs:expr, $rhs:expr) => {
            #[test]
            fn $name() {
                let result1 = parse($lhs).unwrap();
                let result2 = parse($rhs).unwrap();
                assert_eq!(result1, result2);
            }
        };
    }

    case_eq!(test_prec, "1 + 2 * 3", "1 + (2 * 3)");
    case_eq!(test_spaces, "    1 + 2   * 3   ", "1 + 2 * 3");

    macro_rules! case_err {
        ($name:ident, $input:expr) => {
            #[test]
            fn $name() {
                let result = parse($input);
                assert!(result.is_err());
            }
        };
    }

    case_err!(test_invalid_char, "1 + 2a");
    case_err!(test_unmatched_paren, "(1 + 2");
    case_err!(test_empty_input, "");
    case_err!(test_invalid_assignment, "x = ");
    case_err!(test_invalid_expression, "1 + * 2");
}
