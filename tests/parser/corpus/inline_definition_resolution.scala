// teq: --inline-substitution
// What scalac 3.8.4 resolves at an inline method's definition stands at every expansion: the
// type arguments it inferred (`viaLambda[B]`, `viaTuple[B]`) with the givens they found, and the
// overload it chose (`pick(x: B)`), while the dependent result `wrap(x): List[x.type]` is a
// `List[O.type]` after the expansion: the expansion by substitution copies what the definition
// resolved. The retype path, which types the body again at the arguments' own
// types, gives `TC[O.type]` and `object` (`tests/cases/inline_arg_wrapped` pins its answers).
trait B
object O extends B
trait TC[A]:
  def name: String
given TC[B] with
  def name = "TC[B]"
given TC[O.type] with
  def name = "TC[O.type]"
def viaLambda[A](x: () => A)(using tc: TC[A]): String = tc.name
def viaTuple[A](x: (A, Int))(using tc: TC[A]): String = tc.name
inline def f(x: B): String = viaLambda(() => x) + " " + viaTuple((x, 1))
def pick(x: B): String = "base"
def pick(x: O.type): String = "object"
inline def wrap(x: B): List[x.type] = List(x)
inline def g(x: B): String = pick(wrap(x).head)
inline def h(x: B): String = pick(x)
@main def run(): Unit =
  println(f(O))
  println(g(O))
  println(h(O))
