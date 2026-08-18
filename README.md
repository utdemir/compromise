# compromise

> Are we doomed to it, Lord, chained to the pendulum of our own mad clockwork, helpless to halt its swing?[^1]

[^1]: Walter M. Miller Jr., *A Canticle for Leibowitz* ([link](https://www.goodreads.com/quotes/752109-listen-are-we-helpless-are-we-doomed-to-do-it))

LLM generated code is pretty bad. So the only way to make use of them is either reviewing with _more_ scrutiny than usual (which nullifies any "efficiency" gains), or mechanically verifying them.

This library is my (low effort) attempt/suggestion to express the above distinction.

- Write code with verification in mind.
- While writing code - we can leave "gaps" (using the `slop` macro).
- This library rewrites the `#[slop]` annotated functions to call functions under a `zz_slop` directory.
- Prompt/configure your agent to _only_ edit files under `zz_slop`.
- Do not (directly) use `zz_slop` functions - it's only purpose is to get `#[slop]` annotated functions to compile.
- Review the human-written code & specs in detail, don't pay _too much_ attention to the `zz_slop` implementations.

## Example

```rust
use compromise::slop;

#[slop]
fn add(x: i32, y: i32) -> i32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
```

See [./example](./example) for a sligthly more complex example.

## FAQ

- **How to test side-effects?**: Ideally - only mark pure functions with #[slop]. So they can be easily tested. Use the techniques of your trade - functional core/imperative shell, sans-io, dependency injection, etc. all work just as fine. Ideally we would be writing this in a language that can express purity - but Rust ain't it.
- **How to test for performance characteristics?**: I mean there should probably be a (deterministic) benchmark somewhere if performance characteristics matter - but I admit I don't have a good answer for this one.
- **Do I need to read the `zz_slop` implementations?**: Rarely. If there is a bug, the solution would be to express it as a verification test and let LLM fix the implementation again. I do sometimes find it useful to vaguely read the shape of the `zz_slop` directory as it sometimes brings up some awkward semantics with the spec.
- **Should I never use LLM's to edit files outside `zz_slop`?**: Not strictly. Merely means that if you _do_ - you should make sure to review them carefully - as you're adding technical debt a lot more compared to files under `zz_slop`. LLM's does tend to write terrible specs - so I suggest only using it for boilerplaty code.
- **My editor sees a bunch of "undefined function" errors?**: See the "Editor support" section below.
- **Why the name `zz_slop`?**: I wanted it to come last in file listings and GitHub PR review pane.
- **But LLM's also destroy the environment/steal peoples work/spread misinformation/increase the class divide/deskill engineers/introduce massive security risks/lower the quality of products/...**. Yeah. If you can make your living without having to use them - you should. I don't think I can find a well-paying software job without having to see LLM generated code - so I'm trying to make _that part_ palatable.

## Agent configuration

So far - simply using an AGENTS.md with something like:

> When developing this example, you should only edit files under zz_slop/ directory. 
>
> The author has written specification for you to follow. Your task is to implement the files under the zz_slop directory so it passes the specification.
>
> Do not edit files outside zz_slop/ directory without explicit permission from the author.
>
> If the specification is unclear, inconsistent or has unnatural semantics (ie. causes implementation to be awkward), seek clarification from the author.
>
> If minor changes to the clarification would significantly improve the implementation, prompt the author with a suggestion - without making the change yourself.
>
> After your changes, leave the code in a typechecked and tests passing state (unless waiting for clarifications).

worked for me. Someone with more appetite for configuring coding agents should probably come up with a hook/skill/mcp or whatever.

## Editor support

When using this crate - there's a long time period where you're writing the spec but the implementation is missing so your editor will complain a lot. To mitigate this - you can use the `panicking` feature of `compromise` crate so it generates implementations that always compile (while panicking at runtime).

Add the following to your `Cargo.toml`:

```toml
[features]
panicking = ["compromise/panicking"]
```

Add the equivalent of following to your editor configuration (below is for VsCode/rust-analyzer):

```json
{
  "rust-analyzer.cargo.features": ["panicking"]
}
```