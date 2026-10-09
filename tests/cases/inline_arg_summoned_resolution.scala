// A summoned using parameter is the parameter as written for what resolves on it: `pick(summon[B])`
// ranks at the declared `B`, a nested inline binding still takes the argument's own type.
trait B:
  inline def apply(): String
object O extends B:
  inline def apply(): String = "o"
class C extends B:
  inline def apply(): String = "c"
def pick(x: B): String = "base"
def pick(x: O.type): String = "object"
def pick(x: C): String = "class"
trait TC[A]:
  def name: String
given TC[B] with
  def name = "TC[B]"
given TC[O.type] with
  def name = "TC[O.type]"
def named[T](x: T)(using tc: TC[T]): String = tc.name
extension (x: B) def tag: String = "ext-base"
extension (x: O.type) def tag: String = "ext-object"
inline def inner(x: B): String = x()
inline def f(using x: B): String = pick(summon[B]) + " " + named(summon[B]) + " " + summon[B].tag + " " + inner(summon[B])
// scala-library's `summon` spelt in source: a transparent inline result of the parameter's
// singleton type has, inside `g`, the identity of the parameter it was given.
transparent inline def own[T](using x: T): x.type = x
inline def g(using x: B): String = pick(own[B]) + " " + named(own[B]) + " " + own[B].tag + " " + inner(own[B])
// A result type that only mentions the parameter (`List[x.type]`) is the
// call's own type, not the parameter's: `pick(wrap(x))` ranks at `List[B]`, `Any`'s alternative.
inline def wrap(x: B): List[x.type] = List(x)
// An intersection of the singleton with a type the declared type conforms to is the singleton
// (`x.type & B` over `x: B`); with a type below the declared type it is the call's own.
inline def both(x: B): x.type & B = x
inline def narrow(x: B): x.type & C = x.asInstanceOf[x.type & C]
inline def m(x: B): String = pick(both(x)) + " " + named(both(x)) + " " + pick(narrow(x)) + " " + inner(both(x))
// An alias of the singleton is the singleton.
type Same[X <: Singleton] = X
transparent inline def alias[T](using x: T): Same[x.type] = x
inline def k(using x: B): String = pick(alias[B]) + " " + named(alias[B]) + " " + inner(alias[B])
def pick(xs: List[B]): String = "list"
inline def h(x: B): String = pick(wrap(x))
@main def run(): Unit =
  println(f(using O))
  println(f(using new C))
  println(g(using O))
  println(g(using new C))
  println(h(O))
  println(h(new C))
  println(k(using O))
  println(k(using new C))
  println(m(new C))
