# Introduction

Recently I saw a post in [/r/rust](https://reddit.com/r/rust) complaining a bit about the
[Nom](https://github.com/Geal/nom) parser combinator crate
requiring manually threading input through the parsers and I was curious if there was
a way to make writing Nom parsers more similar to Haskell. Monads and `do` notation make
writing parsers in Haskell very ergonomic.


Before I continue, I'd like to add that I don't intend any slight or criticism toward Nom, nor am I implying that using
the combinator functions directly is the only or best approach. I just was interested in solving this particular
issue using `do` notation. I believe Nom also includes macro versions of combinators which may be
more conventient to use.

Anyway, it turns out there's a crate which uses a handy macro to enable something very similar to Haskell's
`do` notation - enter [do-notation](https://github.com/phaazon/do-notation) which actually uses `m!` rather than
`do!` since `do` is a reserved word in Rust.

A simple example with `Option` would be something like this:

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

If you're familiar with Haskell, this would be `return` or `pure`. The last line of the example with
`m!` could also be written as `<_>::lift(x + y)` or `<Option<i32>>::lift(x + y)`.

Also required is an `and_then` function associated with the type you're using as a monad. Rust `std::Option` and `std::Result` already include this by default. Link to [Option::and_then](https://doc.rust-lang.org/std/option/enum.Option.html#method.and_then) -
and you can just view the source for it right there. We'd call this `bind` or `>>=` in Haskell.

Not being a type wizard, I had to work my way toward being able to represent a Nom parser, which is basically a State monad combined with Error. Starting with...

# State

Source: [src/state.rs](src/state.rs)

The approach I used here was precisely the same as the state monad from Haskell - the `State` type holds a closure
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

And to set the state:

```rust
pub fn putst<'a, SNEW: 'a>(snew: SNEW) -> State<'a, SNEW, ()> {
  State(Box::new(move |_| ((), snew)))
}
```

The function to run an action is very simple - we just apply the state to the closure stored inside `State`
to get back a tuple of the return value and last state.

```rust
pub fn run_state<S, A>(s: S, ma: State<S, A>) -> (A, S) {
  ma.0(s)
}
```

Using it looks like this:

```rust
// Type annotation here not actually needed.
let result: State<i32, i32> = m! {
  st <- getst();
  putst(st + 1);
  st <- getst();
  putst(st + 1);
  getst()
};
```

# State Inside Result

Source: [src/stateresult.rs](src/stateresult.rs)

For the State monad, our type looks like:

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

The only difference is now the value is wrapped by `Result`. Since now our actions
can fail, a function to throw an error and abort the computation is included:

```rust
pub fn throwstres<'a, S, A, E: 'a>(e: E) -> StateResult<'a, S, A, E> {
  StateResult(Box::new(|_| Err(e)))
}
```

Using the monad looks like this:

```rust
let result = m! {
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
```

The type for `result` in the previous example would be something like
`StateResult<i32, i32, &'static str>`

Worth noting is that this approach throws away the current state when hitting `Err`. This
way of doing it is most similar to Nom but there's another possibility:

# State Beside Result

Source: [src/stateeither.rs](src/stateeither.rs) - no particular reason for that name.

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

You'd use it something like this

```rust
let (next_input, _result) = tag("abc")(current_input);
```

If `tag` matches, then it consumes that input and returns the value - otherwise it fails.

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

For comparison, the corresponding plain version with manual input threading would look like:

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



# Is It Practical?

I suspect not. There are several pretty big disadvantages:

1. Wrapping/building up closures on every operation is very likely to sap
performance and I wouldn't be surprised if it blows up the stack too.
2. Many Nom parsers take other parsers. If you're trying to use `do` notation, you're
suddenly back in plain Nom parser territory and have to either keep trying wrap to the functions or just write
your parser the old fashioned way starting from that point.
It is possible writing a library of wrapped combinators would alleviate that issue.
3. Code inside macros with special syntax presents a problem for development tools like IDEs, automatic formatting,
etc. It's not clear the benefit of clearer syntax with monadic parsing (if everyone would even agree it is a benefit!)
outweighs that downside.

On the plus side, it's pretty interesting that this is possible and I got further with it than I expected to!

# Closing

I'm sure my approach isn't optimal, even within what's allowed by Rust's type system. I'd be very
interested in seeing potential improvements. There's also a good change my use of lifetimes
is incorrect or suboptimal - I basicalyl got it to the point where my examples would run and then
left it alone.

Feedback welcome! The best way to contact me is via reddit with username `KerfuffleV2`
