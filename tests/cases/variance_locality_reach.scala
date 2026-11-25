// teq: --wconf msg=Ignoring:s
// The `private[this]` class parameter reached as its class's `this` in every form the typer makes the prefix (the
// variance part's fix round, 25 Nov): two and three classes deep through the outer accessors, from an anonymous
// class, a local class and a lambda in each, a by-name argument and a local def, read, assigned and compound
// assigned; and a case class's `private[this]` parameter whose type is `@uncheckedVariance`, which the check of the
// getter of `copy`'s default leaves unchecked as the parameter's own check does. scalac 3.8.4 compiles and runs it.
import scala.annotation.unchecked.uncheckedVariance
trait F { def go: Int }
def by(v: => Int): Int = v
class C(private[this] val x: Int):
  class Inner:
    class Deep:
      def read: Int = C.this.x
    def deep: Int = new Deep().read
  def two: Int = new Inner().deep
  def anon: Int = (new F { def go: Int = (new F { def go: Int = C.this.x }).go }).go
  def local: Int = { class L1 { def go: Int = (() => { class L2 { def go: Int = x }; new L2().go })() }; new L1().go }
  def named: Int = by({ def g(): Int = x + 1; g() })
class V(private[this] var x: Int):
  class Inner:
    class Deep:
      def bump(): Int = { V.this.x += 1; x }
    def deep: Int = new Deep().bump()
  def set(): Int = { (() => { x = 10 })(); new Inner().deep }
case class U1[-T](private[this] val a: T @uncheckedVariance)
case class U2[-T](private[this] val a: List[T @uncheckedVariance])
@main def run(): Unit =
  val c = new C(7)
  println(s"${c.two} ${c.anon} ${c.local} ${c.named}")
  println(new V(1).set())
  println(s"${U1[Int](1).hashCode == U1[Int](1).hashCode} ${U2[Int](List(1)) == U2[Int](List(1))}")
