use compromise::slop;

#[slop]
struct Example;

#[slop]
impl Example {
    fn borrowed(&self) -> &Self;
}

fn main() {}
