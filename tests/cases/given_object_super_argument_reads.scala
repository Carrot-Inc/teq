// Superclass arguments, a trait and a local class in a given object read its member and its this.
trait T
class Parent(val x: Int, val o: AnyRef)
object O {
 given G: T with {
  val n = 42
  class K extends Parent(n, G.this)
  trait R { def get = n; def own: AnyRef = G.this }
  class C extends R
  def local(v: Int): Int = { class L extends Parent(n + v, G.this); new L().x }
 }
}
object Main { def main(args: Array[String]): Unit = {
 val k = new O.G.K
 println(k.x); println(k.o eq O.G); println(new O.G.C().get); println(new O.G.C().own eq O.G); println(O.G.local(1))
} }
