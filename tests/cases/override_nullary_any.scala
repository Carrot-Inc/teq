// A parameterless member overrides an empty-paren one, and the reverse, where either or a member
// either overrides is Java's, Any's, Object.clone or a Scala 2 class's (scalac's
// `matchNullaryLoosely`): `Function1.toString()`, a trait's `toString()`, a `clone()` overridden by
// `clone`, scala-library's `Iterator.hasNext` by `hasNext()`.
class F extends (Int => Int):
  def apply(x: Int): Int = x + 1
  override def toString: String = "F"
trait T:
  def toString(): String
  def hashCode(): Int
class C extends T:
  override def toString: String = "C"
  override def hashCode: Int = 7
class Count(n: Int) extends Iterator[Int]:
  private var i = 0
  def hasNext(): Boolean = i < n
  def next(): Int = { i += 1; i }
class Base:
  override def clone(): AnyRef = "base clone"
class Derived extends Base:
  override def clone: AnyRef = "derived clone"
@main def run(): Unit =
  println(new F)
  println(new F()(1))
  val t: T = new C
  println(t.toString())
  println(t.hashCode())
  println(new Count(3).toList)
  val b: Base = new Derived
  println(b.clone())
