// A parameterless member overrides an empty-paren one where that one overrides a member of Any or
// of Java (scalac's `matchNullaryLoosely`): `Function1.toString()` and a trait's `toString()`.
class F extends (Int => Int):
  def apply(x: Int): Int = x + 1
  override def toString: String = "F"
trait T:
  def toString(): String
  def hashCode(): Int
class C extends T:
  override def toString: String = "C"
  override def hashCode: Int = 7
@main def run(): Unit =
  println(new F)
  println(new F()(1))
  val t: T = new C
  println(t.toString())
  println(t.hashCode())
