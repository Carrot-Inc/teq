// Trait parameters of every kind in the class mixing the trait in, each a field the class sets before the trait's
// `$init$` (dotty's `Mixin`, `traitInits`): a generic one, named arguments out of order (their statements first), a
// using clause after a val's, a private one the trait never reads, a `var` with its setter, a protected one, one of a
// trait a later trait extends, an enum's and an object's, a case class's, a default from the trait's companion.
def tag(s: String): String = { println("tag " + s); s }
trait Box[A](val a: A):
  def show = "box " + a
class IntBox extends Box[Int](41):
  def next = a + 1
trait Named(val first: String, val last: String = "Doe"):
  println("named " + first + " " + last)
class Person extends Named(last = tag("L"), first = tag("F"))
class Default extends Named(tag("D"))
trait WithUsing(val n: Int)(using val ord: Ordering[Int]):
  def max(a: Int) = ord.max(a, n)
class U extends WithUsing(5)
trait Unused(u: Int):
  def hello = "hello"
class UU extends Unused(7)
trait V(var v: Int):
  def bump() = v += 1
class VV extends V(10)
trait P(protected val p: Int):
  def pp = p * 2
class PP extends P(4)
trait A(val a: Int):
  println("A " + a)
trait B extends A:
  def twice = a * 2
class C extends A(tag("c").length) with B
enum E(val code: Int) extends A(code + 100):
  case X extends E(1)
  case Y extends E(2)
object O extends A(tag("o").length) with B
case class CC(z: Int) extends Box[String]("cc" + z)
@main def main(): Unit =
  val b = IntBox()
  println(b.show + " " + b.next + " " + (b: Box[Int]).a)
  val p = Person()
  println(p.first + p.last)
  println(Default().last)
  println(U().max(3) + " " + U().max(9))
  println(UU().hello)
  val vv = VV(); vv.bump(); vv.v = vv.v * 2; println(vv.v)
  println(PP().pp)
  val c = C(); println(c.twice + " " + c.a)
  println(E.X.a + " " + E.Y.code)
  println(O.twice)
  println(CC(2).a + " " + CC(2))
