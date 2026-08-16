use std::io::{self, Write};

mod zz_slop;

mod eval;
mod parser;

fn main() -> io::Result<()> {
    let mut eval = eval::Eval::new();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match eval.eval_str(&input) {
            Ok(eval::EvalOutput::Val(result)) => println!("{}", result),
            Ok(eval::EvalOutput::Lam) => println!("<lambda>"),
            Ok(eval::EvalOutput::Dec) => println!("<declaration>"),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
