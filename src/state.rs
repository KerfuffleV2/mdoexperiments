use do_notation::Lift;

pub struct State<'a, S, A>(Box<dyn FnOnce(S) -> (A, S) + 'a>);

impl<'a, S, A: 'a> Lift<A> for State<'a, S, A> {
  fn lift(a: A) -> Self {
    Self(Box::new(move |s| (a, s)))
  }
}

impl<'a, S, A: 'a> State<'a, S, A> {
  pub fn and_then<B, F>(self, f: F) -> State<'a, S, B>
  where
    F: FnOnce(A) -> State<'a, S, B> + 'a,
    S: 'static,
  {
    let h = self.0;
    State(Box::new(move |s| {
      let (a, newstate) = h(s);
      let g = f(a).0;
      g(newstate)
    }))
  }
}

pub fn getst<'a, S: Clone + 'a>() -> State<'a, S, S> {
  State(Box::new(|s| (s.clone(), s)))
}

pub fn putst<'a, SNEW: 'a>(snew: SNEW) -> State<'a, SNEW, ()> {
  State(Box::new(move |_| ((), snew)))
}

pub fn run_state<S, A>(s: S, ma: State<S, A>) -> (A, S) {
  ma.0(s)
}

#[cfg(test)]
mod tests {
  use super::*;
  use do_notation::m;

  #[test]
  fn testes() {
    let ma = m! {
      st <- getst();
      putst(st + 1);
      st <- getst();
      putst(st + 1);
      getst()
    };
    assert_eq!((12, 12), run_state(10, ma));
  }
}
