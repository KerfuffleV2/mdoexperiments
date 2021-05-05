use do_notation::Lift;

#[allow(clippy::type_complexity)]
pub struct StateResult<'a, S, A, E>(Box<dyn FnOnce(S) -> Result<(S, A), E> + 'a>);

impl<'a, S, A: 'a, E> Lift<A> for StateResult<'a, S, A, E> {
  fn lift(a: A) -> Self {
    Self(Box::new(move |s| Ok((s, a))))
  }
}

impl<'a, S, A: 'a, E> StateResult<'a, S, A, E> {
  pub fn and_then<B, F>(self, f: F) -> StateResult<'a, S, B, E>
  where
    F: FnOnce(A) -> StateResult<'a, S, B, E> + 'a,
    S: 'a,
    E: 'a,
  {
    StateResult(Box::new(move |s| match self.0(s) {
      Ok((newstate, a)) => f(a).0(newstate),
      Err(e) => Err(e),
    }))
  }
}

pub fn getstres<'a, S: Clone, E>() -> StateResult<'a, S, S, E> {
  StateResult(Box::new(|s| Ok((s.clone(), s))))
}

pub fn putstres<'a, SNEW: 'a, E>(snew: SNEW) -> StateResult<'a, SNEW, (), E> {
  StateResult(Box::new(move |_| Ok((snew, ()))))
}

pub fn throwstres<'a, S, A, E: 'a>(e: E) -> StateResult<'a, S, A, E> {
  StateResult(Box::new(|_| Err(e)))
}

pub fn run_state_result<S, A, E>(s: S, ma: StateResult<S, A, E>) -> Result<(S, A), E> {
  ma.0(s)
}

#[cfg(test)]
mod tests {
  use super::*;
  use do_notation::m;

  fn helper<'a>(limit: i32) -> StateResult<'a, i32, i32, &'static str> {
    m! {
      st <- getstres();
      putstres(st + 1);
      st <- getstres();
      putstres(st + 1);
      st <- getstres();
      _ <- if st > limit {
        throwstres("Oh no!")
      } else {
        <_>::lift(())
      };
      getstres()
    }
  }
  #[test]
  fn sadface() {
    assert_eq!(Err("Oh no!"), run_state_result(10, helper(11)));
  }

  #[test]
  fn happyface() {
    assert_eq!(Ok((12, 12)), run_state_result(10, helper(12)));
  }
}
