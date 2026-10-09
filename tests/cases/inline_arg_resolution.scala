// Inside an inline body an argument that names a parameter is ranked among overloads and
// inferred from at the parameter's declared type, as scalac resolved the body at the
// definition, while the parameter itself carries the argument's own type: a transparent
// expansion returning it has that type.
trait B:
  def value: B
object O extends B:
  def value: O.type = this
class C extends B:
  def value: B = this
  def extra = "extra"
trait TC[A]:
  def name: String
given TC[B] with
  def name = "TC[B]"
given TC[O.type] with
  def name = "TC[O.type]"
given TC[C] with
  def name = "TC[C]"
def pick(x: Any): String = "any"
def pick(x: B): String = "base"
def pick(x: O.type): String = "object"
def pick(x: C): String = "class"
def named[T](x: T)(using tc: TC[T]): String = tc.name
inline def f(x: B): String = pick(x) + " " + named(x)
// A member of the parameter is resolved at the declared type (`B.value: B`), so `pick` is the
// overload of `B`, whatever the override the argument's class has (`O.value: O.type`); the call
// names that override, as scalac's selection on the argument does (`O$.value()` on the JVM).
inline def member(x: B): String = pick(x.value)
// A using parameter is a given of its declared type: `x: B` is no `D`, whatever the argument.
trait D:
  def name: String
object OD extends B, D:
  def value: B = this
  def name: String = "object"
given D with
  def name: String = "fallback"
inline def summoned(using x: B): String = summon[D].name + " " + summon[B].value.toString.take(2)
// Through a nested expansion the inner parameter's declared type ranks (`Any`), not the
// outer's (`B`), whose proxy is the inner argument.
inline def inner(x: Any): String = pick(x)
inline def outer(x: B): String = inner(x)
transparent inline def pass(x: B) = x
transparent inline def any(x: Any) = x
// A cast or an ascription of the parameter is not the parameter: it is ranked at its own type,
// a cast to the argument's own type included (scalajs-react's `VdomNode.cast(n: Any) =
// apply(n.asInstanceOf[React.Node])` over a node).
class Node
class Leaf extends Node
class Arr
object V:
  def apply(n: Node): String = "node"
  def apply(l: Leaf): String = "leaf"
  def apply(a: Arr): String = "arr"
  inline def castNode(n: Any): String = apply(n.asInstanceOf[Node])
  inline def castLeaf(n: Node): String = apply(n.asInstanceOf[Leaf])
  inline def castArr(n: Any): String = apply(n.asInstanceOf[Arr])
  inline def ascribed(n: Node): String = apply(n: Node)
  inline def bare(n: Node): String = apply(n)
  inline def parens(n: Node): String = apply((n))
@main def run(): Unit =
  println(f(O))
  println(f(new C))
  val c = new C
  println(f(c))
  println(pass(new C).extra)
  println(any(41) + 1)
  val node: Node = new Node
  val leaf = new Leaf
  println(V.castNode(node))
  println(V.castLeaf(leaf))
  println(V.castArr(new Arr))
  println(V.ascribed(leaf))
  println(V.bare(leaf))
  println(V.parens(leaf))
  println(member(O))
  println(member(new C))
  println(summoned(using OD))
  println(outer(new C))
  println(outer(c))
  println(outer(O))
