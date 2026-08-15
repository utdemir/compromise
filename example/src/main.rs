use std::io::{self, Write};

mod zz_slop;

mod parser;
mod eval;

fn main() -> io::Result<()> {
    let mut eval = eval::Eval::new();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match eval.eval_str(&input) {
            Ok(Some(result)) => println!("{}", result),
            Ok(None) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
