// The subjects of the reflection's probes (README.md, "The reflection's probes"): the members each review pass's probe
// of teq's reflection widens, as the pass wrote them.
package repr.probe

// dc2a3dfec's pass, finding 3: a val of `this.type`, and a nested object under a val's path.
class Self:
  val x: this.type = this
class Outer:
  object Inner
object Model:
  val o = new Outer

// 4f95a8370's pass, finding 3: a polymorphic method whose bound its class's argument gives; finding 4: one name in
// both arms of an intersection.
class C[A]:
  def h[B <: A](x: B): B = x
trait L
trait R
trait HasL { def f: L }
trait HasR { def f: R }

// 3362b74bf's pass, finding 1, and 4f95a8370's, findings 1 to 5.
object Subject:
  def f(x: Int): String = "s"
  def g[A](x: A): A = x
  val a: Int @scala.annotation.internal.RuntimeChecked = 1
  def over(x: Int): Int = x
  def over(x: String): String = x
  def dep(x: AnyRef): x.type = x
  val c: C[String] = new C[String]
  val ab: HasL & HasR = null
  def rep(xs: String*): Int = xs.size
  inline def inl(inline x: Int): Int = x
