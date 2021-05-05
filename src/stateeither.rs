use do_notation::Lift;

#[allow(clippy::type_complexity)]
pub struct StateEither<'a, S, A, E>(Box<dyn FnOnce(S) -> (S, Result<A, E>) + 'a>);

impl<'a, S, A: 'a, E> Lift<A> for StateEither<'a, S, A, E> {
  fn lift(a: A) -> Self {
    Self(Box::new(move |s| (s, Ok(a))))
  }
}

impl<'a, S, A: 'a, E> StateEither<'a, S, A, E> {
  pub fn and_then<B, F>(self, f: F) -> StateEither<'a, S, B, E>
  where
    F: FnOnce(A) -> StateEither<'a, S, B, E> + 'a,
    S: 'a,
    E: 'a,
  {
    StateEither(Box::new(move |s| match self.0(s) {
      (newstate, Ok(a)) => f(a).0(newstate),
      (s, Err(e)) => (s, Err(e)),
    }))
  }
}

pub fn getste<'a, S: Clone, E>() -> StateEither<'a, S, S, E> {
  StateEither(Box::new(|s| (s.clone(), Ok(s))))
}

pub fn putste<'a, SNEW: 'a, E>(snew: SNEW) -> StateEither<'a, SNEW, (), E> {
  StateEither(Box::new(move |_| (snew, Ok(()))))
}

pub fn throwste<'a, S, A, E: 'a>(e: E) -> StateEither<'a, S, A, E> {
  StateEither(Box::new(|s| (s, Err(e))))
}

pub fn run_state_either<S, A, E>(s: S, ma: StateEither<S, A, E>) -> (S, Result<A, E>) {
  ma.0(s)
}

#[cfg(test)]
mod tests {
  use super::*;
  use do_notation::m;

  fn helper<'a>(limit: i32) -> StateEither<'a, i32, i32, &'static str> {
    m! {
      st <- getste();
      putste(st + 1);
      st <- getste();
      putste(st + 1);
      st <- getste();
      _ <- if st > limit {
        throwste("Oh no!")
      } else {
        <_>::lift(())
      };
      getste()
    }
  }
  #[test]
  fn sadface() {
    assert_eq!((12, Err("Oh no!")), run_state_either(10, helper(11)));
  }

  #[test]
  fn happyface() {
    assert_eq!((12, Ok(12)), run_state_either(10, helper(12)));
  }
}
