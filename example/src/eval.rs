use compromise::slop;

#[slop]
pub struct Eval;

#[slop]
impl Eval {
    pub fn new() -> Self;
    pub fn eval_str(&mut self, input: &str) -> Result<Option<i64>, &'static str>;
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

    case_one!(test_addition, "1 + 1", Ok(Some(2)));
    case_one!(test_subtraction, "5 - 3", Ok(Some(2)));
    case_one!(test_multiplication, "2 * 3", Ok(Some(6)));
    case_one!(test_let, "x = 10", Ok(None));
    case_one!(test_variable, "x = 10\nx + 5", Ok(Some(15)));
    case_one!(test_unbound, "y + 1", Err("unbound variable"));

    #[test]
    fn example_scenario() {
        let mut eval = Eval::new();
        assert_eq!(eval.eval_str("x = 10"), Ok(None));
        assert_eq!(eval.eval_str("y = 5"), Ok(None));
        assert_eq!(eval.eval_str("x + y"), Ok(Some(15)));
        assert_eq!(eval.eval_str("z + x"), Err("unbound variable"));
        assert_eq!(eval.eval_str("z = 20"), Ok(None));
        assert_eq!(eval.eval_str("z +"), Err("syntax error"));
        assert_eq!(eval.eval_str("z + x"), Ok(Some(30)));
    }
}
