// A dependent function's result approximated without its parameter (`nonDependentResultApprox`'s
// `ApproximatingTypeMap`) maps an abstract member's application at the variances its parameters are declared with
// (`mapArgs(args, tyconTypeParams(tp))`, an abstract member's `typeParams` its declared ones, `boundsFromParams`):
// `c.FC[c.Ab]` of a covariant `FC` takes `c.Ab`'s upper end, so the function is an `A => A#FC[String]`.
class A:
  type Ab >: Null <: String
  type FC[+X]
def h(f: (c: A) => c.FC[c.Ab]): A => A#FC[String] = f

@main def run(): Unit =
  val g = h(c => null.asInstanceOf[c.FC[c.Ab]])
  println(g(A()) == null)
