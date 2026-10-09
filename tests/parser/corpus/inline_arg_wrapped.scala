// A parameter inside a wrapper consumed for inference, a lambda's result or a tuple's element,
// infers the argument's own type: `TC[O.type]` twice here, where scalac, typing the body once
// at the definition, infers the declared `TC[B]`. These are the retype
// path's answers, the default's until the expansion by substitution is the default;
// `tests/cases/inline_arg_wrapped_substitution` is this program under `--inline-substitution`,
// with scalac's `TC[B] TC[B]` and `base`.
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
// `List[O.type]` once expanded. teq's expansion ranks the overloads again at the argument's own
// type: `object`.
def pick(x: B): String = "base"
def pick(x: O.type): String = "object"
inline def wrap(x: B): List[x.type] = List(x)
inline def g(x: B): String = pick(wrap(x).head)
@main def run(): Unit =
  println(f(O))
  println(g(O))
