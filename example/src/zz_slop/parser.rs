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
    Pipe,
    Comma,
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
            '|' => tokens.push(Token::Pipe),
            ',' => tokens.push(Token::Comma),
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
        if matches!(self.peek(), Some(Token::Pipe)) {
            return self.lambda();
        }
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
        let mut left = self.application()?;
        loop {
            left = match self.peek() {
                Some(Token::Star) => {
                    self.position += 1;
                    Expr::Mul(Box::new(left), Box::new(self.application()?))
                }
                Some(Token::Slash) => {
                    self.position += 1;
                    Expr::Div(Box::new(left), Box::new(self.application()?))
                }
                _ => return Ok(left),
            };
        }
    }

    fn lambda(&mut self) -> Result<Expr, &'static str> {
        self.position += 1;
        let mut parameters = Vec::new();
        loop {
            match self.tokens.get(self.position).cloned() {
                Some(Token::Ident(name)) => {
                    parameters.push(name);
                    self.position += 1;
                }
                _ => return Err("syntax error"),
            }
            match self.peek() {
                Some(Token::Comma) => self.position += 1,
                Some(Token::Pipe) => {
                    self.position += 1;
                    break;
                }
                _ => return Err("syntax error"),
            }
        }

        let mut body = self.expression()?;
        for parameter in parameters.into_iter().rev() {
            body = Expr::Lam(parameter, Box::new(body));
        }
        Ok(body)
    }

    fn application(&mut self) -> Result<Expr, &'static str> {
        let mut expression = self.primary()?;
        while matches!(self.peek(), Some(Token::LeftParen)) {
            self.position += 1;
            let argument = self.expression()?;
            if !matches!(self.peek(), Some(Token::RightParen)) {
                return Err("syntax error");
            }
            self.position += 1;
            expression = Expr::App(Box::new(expression), Box::new(argument));
        }
        Ok(expression)
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
