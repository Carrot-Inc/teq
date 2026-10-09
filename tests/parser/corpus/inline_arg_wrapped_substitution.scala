// teq: --inline-substitution
// `tests/cases/inline_arg_wrapped` under the expansion by substitution: a parameter inside a
// wrapper consumed for inference infers the declared `B` as scalac, typing the body once at the
// definition, infers it (`TC[B]` twice), and the overload the body calls is the one chosen at the
// definition (`pick(x: B)`, `base`), where the retype path gives `TC[O.type]` and `object`.
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
// scalac chooses the overload a body calls at the definition: `pick(wrap(x).head)` over `x: B`
// is `pick(x: B)`, `base`, although the dependent result `wrap(x): List[x.type]` is a
// `List[O.type]` once expanded.
def pick(x: B): String = "base"
def pick(x: O.type): String = "object"
inline def wrap(x: B): List[x.type] = List(x)
inline def g(x: B): String = pick(wrap(x).head)
@main def run(): Unit =
  println(f(O))
  println(g(O))
