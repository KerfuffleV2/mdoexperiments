# Introduction

As a long time Haskell developer recently delving into Rust, something I've really missed
is monads and `do` notation. One thing monadic `do` notation makes particularly nice is
writing parsers. This article will explore the possibility of enabling Haskell-style `do`
notation for writing parsers in Rust.

Probably the most popular crate for parsing in Rust is called [Nom](https://github.com/Geal/nom)
and its parser combinator functions requires manually threading the parser input.
To be fair, I should mention that Nom also provides macro-based parser combinators and
a `do_parse!` macro which is pretty similar to `do` notation in function.

There are also several crates which attempt to allow `do` notation using macros.
I picked one called [do-notation](https://github.com/phaazon/do-notation) that seemed to take
a general approach and had also been updated relatively recently. Note: `do-notation` uses `m!`
rather than `do!` since `do` is a reserved word in Rust.

A simple example with `Option` (known as `Maybe` in the Haskell world) would be something like this:

```rust
// Type annotation included just for clarity.
let result: Option<i32> = m! {
  x <- Some(1);
  let y = 2;
  Some(x + y)
};
```

Bind, `let` and expressions are allowed within `m!`, although pattern matching the left side of a bind doesn't
seem to work. The block needs to have an expression at the end and evaluate to the monad you're using.

To enable using a type with the `m!` macro, it needs to implement the `Lift` trait. Example for `Option`:

```rust
impl <INNER> Lift for Option<INNER> {
  fn lift<inner: INNER> -> Self {
    Some(inner)
  }
}
```

If you're familiar with Haskell, this would be `return` or `pure`. `Some` in previous example could
be written as `<_>::lift` (automatically inferring the type) or `<Option<i32>>::lift`.

Also required is an `and_then` associated function for the type you're planning to use with `m!`.
Rust's `std::Option` and `std::Result` already include this by default. Here is a link to
[Option::and_then](https://doc.rust-lang.org/std/option/enum.Option.html#method.and_then)
and you can just view the source for it right there. We'd call this `bind` or `>>=` in Haskell.

Nom parsers are just state combined with the ability to fail, so let's work our way toward
building a monad that will combine those traits. Starting with...

# State

Source: [src/state.rs](src/state.rs)

The approach I used here was precisely the same as the state monad from Haskell — the `State` type holds a closure
which takes the current state and returns a tuple of the new state and the result of the action. The type
looks like this:

```rust
pub struct State<'a, S, A>(Box<dyn FnOnce(S) -> (A, S) + 'a>);
```

In line with the Haskell state monad, I included a function to extract the current state:

```rust
pub fn getst<'a, S: Clone + 'a>() -> State<'a, S, S> {
  State(Box::new(|s| (s.clone(), s)))
}
```

And one to set the state:

```rust
pub fn putst<'a, SNEW: 'a>(snew: SNEW) -> State<'a, SNEW, ()> {
  State(Box::new(move |_| ((), snew)))
}
```

The function to run a state action is very simple — we just apply the state to the closure stored inside `State`
to get back a tuple of the return value and last state.

```rust
pub fn run_state<S, A>(s: S, ma: State<S, A>) -> (A, S) {
  ma.0(s)
}
```

Using it looks like this:

```rust
// Type annotation here not actually needed.
let action: State<i32, i32> = m! {
  st <- getst();
  putst(st + 1);
  st <- getst();
  putst(st + 1);
  getst()
};
let result = run_state(10, action);
```

`result` here would be `(12, 12)`

# State Inside Result

Source: [src/stateresult.rs](src/stateresult.rs)

For the `State` + `Result` monad, our type looks like:

```rust
pub struct StateResult<'a, S, A, E>(Box<dyn FnOnce(S) -> Result<(S, A), E> + 'a>);
```

Instead of simply returning the state and return value, we return `Result` with either state and
return value or an error. The state manipulation functions haven't changed much:

```rust
pub fn getstres<'a, S: Clone, E>() -> StateResult<'a, S, S, E> {
  StateResult(Box::new(|s| Ok((s.clone(), s))))
}

pub fn putstres<'a, SNEW: 'a, E>(snew: SNEW) -> StateResult<'a, SNEW, (), E> {
  StateResult(Box::new(move |_| Ok((snew, ()))))
}
```

The only difference is now the value is wrapped by `Result`. Since our actions
can fail, I included a function to throw an error and abort the computation:

```rust
pub fn throwstres<'a, S, A, E: 'a>(e: E) -> StateResult<'a, S, A, E> {
  StateResult(Box::new(|_| Err(e)))
}
```

Using the monad looks like this:

```rust
let action = m! {
  st <- getstres();
  putstres(st + 1);
  st <- getstres();
  putstres(st + 1);
  st <- getstres();
  // st is 12 here, so of course this will always fail.
  _ <- if st > 11 {
    throwstres("Oh no!")
  } else {
    <_>::lift(())
  };
  getstres()
};
let result = run_state_result(10, action);
```

The type for `result` in the previous example would be something like
`StateResult<i32, i32, &'static str>`

Worth noting is that this approach throws away the current state when hitting `Err`. This
way of doing it is most similar to Nom but there's another possibility:

# State Beside Result

Source: [src/stateeither.rs](src/stateeither.rs) — no particular reason for that name.

In this case, the type looks like:

```rust
pub struct StateEither<'a, S, A, E>(Box<dyn FnOnce(S) -> (S, Result<A, E>) + 'a>);
```

The implementation is very similar, but we always get the last state back
along with the result, whether or not it is `Err`.

# NomParser!

Source: [src/nomparser.rs](src/nomparser.rs)

Now I can put together what I've learned and implement the required support for using
`do` notation with Nom.

The type of a Nom parser looks like this:

```rust
pub struct NomParser<'a, S, A, E = Error<S>>(Box<dyn FnOnce(S) -> IResult<S, A, E> + 'a>);
```

This is basically the same as `StateResult` from before. The state (`S`) is the data that we're parsing, the result
(`A`) is the result of our parser operations and of course parsing is fallible so there's `E`. Nom parser combinators
actually work very similar to the closure we store inside the `State` or `NomParser` type: they
return a closure that takes the "state" (`input`). One simple example is the `tag` combinator which
just expects a certain value in the input.

Example usage:

```rust
let (next_input, _result) = tag("abc")(current_input)?;
```

If `tag` matches, it consumes the input and returns the matched value — otherwise it fails.

And of course the next time you wanted to apply a parser combinator, you'd pass the input state that `tag` had
returned and recieve the next one plus the return value and so on, which is a bit of a pain.


I added a function to wrap an existing Nom combinator with `NomParser`:

```rust
pub fn nomp<'a, F, S, A, E>(f: F) -> NomParser<'a, S, A, E>
where
  F: FnOnce(S) -> IResult<S, A, E> + 'a,
{
  NomParser(Box::new(f))
}
```

The type matches up exactly with our state eating closure, which isn't an accident.

It's also possible to write a pre-wrapped version of a combinator so we don't have to
write `nomp(tag(etc))` every single time:

```rust

  fn ptag<'a, S, T>(t: T) -> NomParser<'a, S, S>
  where
    S: 'a + nom::InputTake + nom::Compare<T>,
    T: 'a + Clone + nom::InputLength,
  {
    nomp(tag(t))
  }
```

For convenience I included a type alias for parsing `&str` using a standard error type:

```rust
pub type NompStr<'a, A, S = &'a str, E = Error<S>> = NomParser<'a, S, A, E>;
```

Let's take the example from [Nom's README](https://github.com/Geal/nom) and show
what it looks like with `do` notation:

```rust
#[derive(Debug, PartialEq)]
pub struct Color {
  pub red: u8,
  pub green: u8,
  pub blue: u8,
}

fn from_hex(input: &str) -> Result<u8, std::num::ParseIntError> {
  u8::from_str_radix(input, 16)
}

fn is_hex_digit(c: char) -> bool {
  c.is_digit(16)
}

fn hex_primary_parser<'a>() -> NompStr<'a, u8> {
  nomp(map_res(take_while_m_n(2, 2, is_hex_digit), from_hex))
}

fn hex_color_parser<'a>() -> NompStr<'a, Color> {
  m! {
    nomp(tag("#"));
    red <- hex_primary_parser();
    green <- hex_primary_parser();
    blue <- hex_primary_parser();
    <_>::lift(Color { red, green, blue })
  }
}
```

For comparison, the corresponding non-monadic approach with manual input threading would look like:

```rust
// Helper functions and type elided

fn hex_primary(input: &str) -> IResult<&str, u8> {
  map_res(
    take_while_m_n(2, 2, is_hex_digit),
    from_hex
  )(input)
}

fn hex_color(input: &str) -> IResult<&str, Color> {
  let (input, _) = tag("#")(input)?;
  let (input, red) = hex_primary(input)?;
  let (input, green) = hex_primary(input)?;
  let (input, blue) = hex_primary(input)?;
  Ok((input, Color { red, green, blue }))
}
```

**Note**: The example provided in the Nom repo uses the `tuple` combinator to avoid three separate
calls to `hex_primary`.

# Is It Practical?

I suspect not. There are several apparent disadvantages:

1. Wrapping/building up closures on every operation is very likely to sap
performance and I wouldn't be surprised if it blows up the stack too.

2. Many Nom parsers take other parsers. When trying to use `do` notation, you're
suddenly back in plain Nom combinator territory and have to either re-enter `NomParser`
or just write the rest of your parser the old fashioned way.
It is possible writing a library of wrapped combinators would alleviate the issue.

3. Code inside macros with special syntax presents a problem for development tools like IDEs, automatic formatting,
etc. It's not clear the benefit of clearer syntax with monadic parsing (if everyone would even agree it is a benefit!)
outweighs that downside.

On the plus side, it's pretty interesting that this is possible and I got further with it than I expected to!

# Project

This project includes a binary that just runs some random test code. You can execute it using
`cargo run`.

The different monads have a simple test block, which you can run with `cargo test`.

The project also works as a library and can be imported. It exports each module by name, and also
`mdoexperiments::prelude` which pulls in everything and re-exports it.


# Closing

I'm sure my approach isn't optimal, even within what's allowed by Rust's type system. I'd be very
interested in seeing potential improvements. There's also a good change my use of lifetimes
is incorrect or suboptimal — I only got it to the point where my examples would run.

Feedback welcome! The best way to contact me is via reddit with username `KerfuffleV2`
