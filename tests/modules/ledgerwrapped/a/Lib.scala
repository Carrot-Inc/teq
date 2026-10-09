package lwa

// scalac 3.8.4's inline rules, through a product: the body resolves what it calls once, at
// its definition (`TC[B]` inferred through a lambda and a tuple, the overload `pick(x: B)`),
// whatever the argument a downstream's expansion passes.
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
