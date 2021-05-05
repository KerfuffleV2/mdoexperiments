use do_notation::Lift;
use nom::{error::Error, IResult};

pub struct NomParser<'a, S, A, E = Error<S>>(Box<dyn FnOnce(S) -> IResult<S, A, E> + 'a>);

pub type NompStr<'a, A, S = &'a str, E = Error<S>> = NomParser<'a, S, A, E>;

impl<'a, S, A: 'a, E> Lift<A> for NomParser<'a, S, A, E> {
  fn lift(a: A) -> Self {
    Self(Box::new(move |s| Ok((s, a))))
  }
}

impl<'a, S, A, E> NomParser<'a, S, A, E> {
  pub fn and_then<B, F>(self, f: F) -> NomParser<'a, S, B, E>
  where
    F: FnOnce(A) -> NomParser<'a, S, B, E> + 'a,
    A: 'a,
    E: 'a,
    S: 'a,
  {
    NomParser(Box::new(move |s| match self.0(s) {
      Ok((newstate, a)) => f(a).0(newstate),
      Err(e) => Err(e),
    }))
  }
}

pub fn nomp<'a, F, S, A, E>(f: F) -> NomParser<'a, S, A, E>
where
  F: FnOnce(S) -> IResult<S, A, E> + 'a,
{
  NomParser(Box::new(f))
}

pub fn run_nomparser<S, A, E>(s: S, ma: NomParser<S, A, E>) -> IResult<S, A, E> {
  ma.0(s)
}

#[cfg(test)]
mod tests {
  use super::*;
  use do_notation::m;
  use nom::{
    bytes::complete::tag,
    error::{Error, ErrorKind},
    Finish,
  };

  fn ptag<'a, S, T>(t: T) -> NomParser<'a, S, S>
  where
    S: 'a + nom::InputTake + nom::Compare<T>,
    T: 'a + Clone + nom::InputLength,
  {
    nomp(tag(t))
  }

  #[test]
  fn happyface() {
    let ma: NomParser<_, _> = m! {
      // Same as: nomp(tag("hi"))
      ptag("hi");
      ptag("the");
      <_>::lift("Yay!")
    };
    assert_eq!(Ok(("re", "Yay!")), run_nomparser("hithere", ma));
  }

  #[test]
  fn sadface() {
    let ma = m! {
      ptag("hi");
      ptag("thez");
      <_>::lift("Yay!")
    };
    let result = run_nomparser("hithere", ma).finish();
    assert_eq!(
      Err(Error {
        input: "there",
        code: ErrorKind::Tag
      }),
      result
    );
  }
}
