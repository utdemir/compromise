use compromise::slop;

#[slop]
pub struct Eval;

#[slop]
impl Eval {
    pub fn new() -> Self;
    pub fn eval_str(&mut self, input: &str) -> Result<EvalOutput, &'static str>;
}

#[derive(Debug, PartialEq)]
pub enum EvalOutput {
    Val(i64),
    Lam,
    Dec,
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! case_one {
        ($name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let mut eval = Eval::new();
                assert_eq!(eval.eval_str($input), $expected);
            }
        };
    }

    fn val(v: i64) -> Result<EvalOutput, &'static str> {
        Ok(EvalOutput::Val(v))
    }
    fn lam() -> Result<EvalOutput, &'static str> {
        Ok(EvalOutput::Lam)
    }
    fn dec() -> Result<EvalOutput, &'static str> {
        Ok(EvalOutput::Dec)
    }
    fn err(msg: &'static str) -> Result<EvalOutput, &'static str> {
        Err(msg)
    }

    case_one!(test_addition, "1 + 1", val(2));
    case_one!(test_subtraction, "5 - 3", val(2));
    case_one!(test_multiplication, "2 * 3", val(6));
    case_one!(test_let, "x = 10", dec());
    case_one!(test_variable, "x = 10\nx + 5", val(15));
    case_one!(test_unbound, "y + 1", err("unbound variable"));
    case_one!(test_lambda, "|x| x + 1", lam());
    case_one!(test_let_lam, "f = |x| x + 1", dec());
    case_one!(test_call, "(|x| x + 1)(5)", val(6));

    #[test]
    fn example_scenario() {
        let mut eval = Eval::new();
        assert_eq!(eval.eval_str("x = 10"), dec());
        assert_eq!(eval.eval_str("y = 5"), dec());
        assert_eq!(eval.eval_str("x + y"), val(15));
        assert_eq!(eval.eval_str("z + x"), err("unbound variable"));
        assert_eq!(eval.eval_str("z = 20"), dec());
        assert_eq!(eval.eval_str("z +"), err("syntax error"));
        assert_eq!(eval.eval_str("z + x"), val(30));
    }
}
