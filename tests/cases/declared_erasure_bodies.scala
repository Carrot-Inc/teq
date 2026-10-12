// The bodies over declarations whose erasure is what they state beyond their types (the erasure provenance E1):
// a `*:` chain behind an alias erases to `Product`, a union of a value class with itself to `Object`, and a call, a
// field read, an override with the same declaration, a result read as a tuple, a default and an extension adapt.
class V(val x: Int) extends AnyVal
object Types:
  type C = Int *: String *: EmptyTuple
  type U = V | V
case class P(chain: Types.C, u: V = new V(1))
class Base:
  def alias(x: Types.C): Types.C = x
  def union(x: Types.U): Types.U = x
class Sub extends Base:
  override def alias(x: Types.C): Types.C = (x._1 + 1, x._2)
  override def union(x: Types.U): Types.U = new V(x.x + 1)
class Base2:
  def r: Types.C = (1, "x")
  def s: (Int, String) = (1, "s")
class Sub2 extends Base2:
  override def r: (Int, String) = (2, "y")
  override def s: Types.C = (2, "t")
trait Tr:
  def t(x: Types.C): Types.C
class Impl extends Tr:
  def t(x: Types.C): Types.C = x
object Ext:
  extension (x: Types.C) def first: Int = x._1
  def default(x: Types.C = (0, "z")): Types.C = x
@main def run(): Unit =
  val p = P((1, "a"))
  println(p.copy(chain = (2, "b")))
  val P(c, u) = p
  println(c._2 + u.x)
  val b: Base = new Sub
  println(b.alias((3, "c")))
  println(b.union(new V(4)).x)
  val b2: Base2 = new Sub2
  println(b2.r)
  println(b2.s)
  val t: Tr = new Impl
  println(t.t((5, "d")))
  import Ext.*
  println((6, "e").first)
  println(default())
