package sharedlib.optics

type Getter[S, A] = S => A

case class Lens[S, A](get: S => A, set: (S, A) => S):
  def modify(s: S)(f: A => A): S = set(s, f(get(s)))

case class Iso[A, B](to: A => B, from: B => A)
