// A def whose `=` is missing after its result type: the type runs on into the body, and the
// class's member of an unknown signature is no member it fails to implement.
final class L[-R, +E, +O](val build: () => List[Any]):
  def ++[E1 >: E, R2, O2](that: => L[R2, E1, O2]): L[R & R2, E1, O & O2]  L(() => build() ++ that.build())
