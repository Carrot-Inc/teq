// A generic trait's members that a superclass implements under another erasure, givens among them: the class
// mixing the trait in bridges each (dotty's `Bridges`), `g()Object` to the inherited given `g()I`, a given with a
// using clause too. master bridged the def and the val and threw `AbstractMethodError` at `g()`.
trait Gen[A]:
  def d: A
  def v: A
  def g: A
  def gp(using s: String): A
class Base:
  def d: Int = 1
  val v: Int = 2
  given g: Int = 3
  given gp(using s: String): Int = s.length
class C extends Base with Gen[Int]
@main def main(): Unit =
  val c: Gen[Int] = C()
  println(c.d + c.v + c.g + c.gp(using "abcd"))
