pub mod parser {
    use crate::parser::{Expr, Stmt};

    #[derive(Clone, Debug, PartialEq)]
    enum Token {
        Number(i64),
        Ident(String),
        Plus,
        Minus,
        Star,
        Slash,
        Equal,
        LeftParen,
        RightParen,
    }

    pub fn parse(input: &str) -> Result<Stmt, &'static str> {
        let tokens = tokenize(input)?;
        if tokens.is_empty() {
            return Err("syntax error");
        }

        let mut parser = Parser {
            tokens,
            position: 0,
        };
        let statement = parser.statement()?;
        if parser.position != parser.tokens.len() {
            return Err("syntax error");
        }
        Ok(statement)
    }

    fn tokenize(input: &str) -> Result<Vec<Token>, &'static str> {
        let mut chars = input.char_indices().peekable();
        let mut tokens = Vec::new();

        while let Some((start, ch)) = chars.next() {
            match ch {
                ch if ch.is_whitespace() => {}
                '0'..='9' => {
                    let mut end = start + ch.len_utf8();
                    while let Some(&(index, next)) = chars.peek() {
                        if !next.is_ascii_digit() {
                            break;
                        }
                        chars.next();
                        end = index + next.len_utf8();
                    }
                    let value = input[start..end].parse().map_err(|_| "syntax error")?;
                    tokens.push(Token::Number(value));
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let mut end = start + ch.len_utf8();
                    while let Some(&(index, next)) = chars.peek() {
                        if !next.is_ascii_alphanumeric() && next != '_' {
                            break;
                        }
                        chars.next();
                        end = index + next.len_utf8();
                    }
                    tokens.push(Token::Ident(input[start..end].to_owned()));
                }
                '+' => tokens.push(Token::Plus),
                '-' => tokens.push(Token::Minus),
                '*' => tokens.push(Token::Star),
                '/' => tokens.push(Token::Slash),
                '=' => tokens.push(Token::Equal),
                '(' => tokens.push(Token::LeftParen),
                ')' => tokens.push(Token::RightParen),
                _ => return Err("syntax error"),
            }
        }

        Ok(tokens)
    }

    struct Parser {
        tokens: Vec<Token>,
        position: usize,
    }

    impl Parser {
        fn statement(&mut self) -> Result<Stmt, &'static str> {
            if let (Some(Token::Ident(name)), Some(Token::Equal)) =
                (self.tokens.first(), self.tokens.get(1))
            {
                let name = name.clone();
                self.position = 2;
                return Ok(Stmt::Let(name, self.expression()?));
            }
            Ok(Stmt::Eval(self.expression()?))
        }

        fn expression(&mut self) -> Result<Expr, &'static str> {
            let mut left = self.term()?;
            loop {
                left = match self.peek() {
                    Some(Token::Plus) => {
                        self.position += 1;
                        Expr::Add(Box::new(left), Box::new(self.term()?))
                    }
                    Some(Token::Minus) => {
                        self.position += 1;
                        Expr::Sub(Box::new(left), Box::new(self.term()?))
                    }
                    _ => return Ok(left),
                };
            }
        }

        fn term(&mut self) -> Result<Expr, &'static str> {
            let mut left = self.primary()?;
            loop {
                left = match self.peek() {
                    Some(Token::Star) => {
                        self.position += 1;
                        Expr::Mul(Box::new(left), Box::new(self.primary()?))
                    }
                    Some(Token::Slash) => {
                        self.position += 1;
                        Expr::Div(Box::new(left), Box::new(self.primary()?))
                    }
                    _ => return Ok(left),
                };
            }
        }

        fn primary(&mut self) -> Result<Expr, &'static str> {
            let token = self.tokens.get(self.position).cloned();
            self.position += 1;
            match token {
                Some(Token::Number(value)) => Ok(Expr::Num(value)),
                Some(Token::Ident(name)) => Ok(Expr::Var(name)),
                Some(Token::LeftParen) => {
                    let expression = self.expression()?;
                    if !matches!(self.peek(), Some(Token::RightParen)) {
                        return Err("syntax error");
                    }
                    self.position += 1;
                    Ok(expression)
                }
                _ => Err("syntax error"),
            }
        }

        fn peek(&self) -> Option<&Token> {
            self.tokens.get(self.position)
        }
    }
}

pub mod eval {
    use std::collections::HashMap;

    use crate::parser::{Expr, Stmt};

    pub struct Eval {
        variables: HashMap<String, i64>,
    }

    pub fn new() -> Eval {
        Eval {
            variables: HashMap::new(),
        }
    }

    pub fn eval_str(eval: &mut Eval, input: &str) -> Result<Option<i64>, &'static str> {
        let mut result = None;
        let mut saw_statement = false;

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }
            saw_statement = true;
            match crate::zz_slop::parser::parse(line)? {
                Stmt::Let(name, expression) => {
                    let value = eval_expr(eval, &expression)?;
                    eval.variables.insert(name, value);
                    result = None;
                }
                Stmt::Eval(expression) => result = Some(eval_expr(eval, &expression)?),
            }
        }

        if saw_statement {
            Ok(result)
        } else {
            Err("syntax error")
        }
    }

    fn eval_expr(eval: &Eval, expression: &Expr) -> Result<i64, &'static str> {
        match expression {
            Expr::Num(value) => Ok(*value),
            Expr::Var(name) => eval.variables.get(name).copied().ok_or("unbound variable"),
            Expr::Add(left, right) => Ok(eval_expr(eval, left)? + eval_expr(eval, right)?),
            Expr::Sub(left, right) => Ok(eval_expr(eval, left)? - eval_expr(eval, right)?),
            Expr::Mul(left, right) => Ok(eval_expr(eval, left)? * eval_expr(eval, right)?),
            Expr::Div(left, right) => {
                let divisor = eval_expr(eval, right)?;
                if divisor == 0 {
                    return Err("division by zero");
                }
                Ok(eval_expr(eval, left)? / divisor)
            }
        }
    }
}
