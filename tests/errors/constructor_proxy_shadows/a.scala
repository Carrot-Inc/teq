// A constructor proxy's application that shadows an outer binding with a method alternative, or with an
// `apply` that is a method, is scalac's E177 (dotty's `Typer.typedIdent.checkNotShadowed`, the lookup
// repeated without the `PhantomSymbol`), the method inapplicable to the arguments too; a plain `val` or
// `var` does not block it (V), and outside an application the name is the shadowed binding.
package p
def C(x: String): String = x
object O:
  class C(x: Int)
  val c = C(1)
object Q:
  val D = (x: String) => x
  class In:
    class D(x: Int)
    val d = D(1)
  var V = 1
  object W:
    class V(x: Int)
    val v = V(1)
    val w: Int = V
object L:
  def f =
    def E(s: String) = s
    def g =
      class E(x: Int)
      E(1)
    g
object Outer:
  def M(x: String): String = x
  object A { def apply(x: String): String = x }
  def I: Int = 1
  class Inner:
    class M(x: Int)
    class A(x: Int)
    class I(x: String)
    val m = M(1)
    val a = A(1)
    val i = I("s")
// expect: a.scala:9:11: error: Reference to constructor proxy for class C in object O
// expect: shadows outer reference to method C in package p
// expect: a.scala:14:13: error: Reference to constructor proxy for class D in class In
// expect: shadows outer reference to value D in object Q
// expect: a.scala:25:7: error: Reference to constructor proxy for class E
// expect: shadows outer reference to method E
// expect: a.scala:35:13: error: Reference to constructor proxy for class M in class Inner
// expect: shadows outer reference to method M in object Outer
// expect: a.scala:36:13: error: Reference to constructor proxy for class A in class Inner
// expect: shadows outer reference to object A in object Outer
// expect: a.scala:37:13: error: Reference to constructor proxy for class I in class Inner
// expect: shadows outer reference to method I in object Outer
