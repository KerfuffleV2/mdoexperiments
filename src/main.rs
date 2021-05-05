use do_notation::{m, Lift};

use mdoexperiments::prelude::*;

fn test_state() {
  let ma = m! {
    st <- getst();
    let _ = println!("got: {}", st);
    putst(st + 1);
    st <- getst();
    let _ = println!("got: {}", st);
    putst(st + 1);
    getst()
  };
  let result = run_state(10, ma);
  println!("return={:?}, state={:?}", result.0, result.1);
}

fn test_state_either() {
  let ma = m! {
    st <- getste();
    let _ = println!("got: {}", st);
    putste(st + 1);
    st <- getste();
    let _ = println!("got: {}", st);
    putste(st + 1);
    st <- getste();
    _ <- if st > 11 {
      throwste("Oh no!")
    } else {
      <_>::lift(())
    };
    let _ = println!("Still going.");
    getste()
  };
  let result = run_state_either(10, ma);
  println!("Result: {:?}", result);
}

use nom::bytes::complete::tag;

fn test_nom() {
  let ma: NomParser<_, _, nom::error::Error<_>> = m! {
    nomp(tag("hi"));
    nomp(tag("thez"));
    <_>::lift("Yay!")
  };
  let result = run_nomparser("hithere", ma);
  println!("Result: {:?}", result);
}

// See https://github.com/Geal/nom for the normal version.
mod nom_example {
  use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::map_res,
    sequence::tuple,
    IResult,
  };

  use do_notation::{m, Lift};

  use mdoexperiments::nomparser::*;

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

  fn hex_primary(input: &str) -> IResult<&str, u8> {
    map_res(take_while_m_n(2, 2, is_hex_digit), from_hex)(input)
  }

  #[allow(dead_code)]
  // Alternative approach.
  fn hex_primary2(input: &str) -> IResult<&str, u8> {
    run_nomparser(
      input,
      nomp(map_res(take_while_m_n(2, 2, is_hex_digit), from_hex)),
    )
  }

  fn hex_color_parser<'a>() -> NompStr<'a, Color> {
    m! {
      nomp(tag("#"));
      // This is necessary because pattern matching the tuple in bind doesn't seem to work.
      x <- nomp(tuple((hex_primary, hex_primary, hex_primary)));
      let (red, green, blue) = x;
      <_>::lift(Color { red, green, blue })
    }
  }

  fn hex_primary_parser<'a>() -> NompStr<'a, u8> {
    nomp(map_res(take_while_m_n(2, 2, is_hex_digit), from_hex))
  }

  // Alternative approach.
  fn hex_color_parser2<'a>() -> NompStr<'a, Color> {
    m! {
      nomp(tag("#"));
      red <- hex_primary_parser();
      green <- hex_primary_parser();
      blue <- hex_primary_parser();
      <_>::lift(Color { red, green, blue })
    }
  }

  pub fn parse_color() {
    let result = run_nomparser("#2F14DF", hex_color_parser());
    println!("Result: {:?}", result);
    let result = run_nomparser("#2F14DF", hex_color_parser2());
    println!("Result: {:?}", result);
    // Giving it something without the `static lifetime.
    let mut x = String::new();
    x.push_str("#2F14DF");
    let result = run_nomparser(x.as_str(), hex_color_parser());
    println!("Result: {:?}", result);
  }
}

fn main() {
  test_state();
  println!("\n===============\n");
  test_state_either();
  println!("\n===============\n");
  test_nom();
  nom_example::parse_color();
}
