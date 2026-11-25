// A value class's method defaults whose getters are not a method of the class with its own `m$extension`: an inline
// member's default (the getter is an ordinary method of the class, on the companion though the inline method is not),
// and a default inherited from a universal trait's member the class overrides without one (the trait's getter, called on
// the boxed receiver).
trait T extends Any:
  def m(a: Int)(b: Int = a + 1): Int = a * 10 + b

class V(val u: Int) extends AnyVal with T:
  inline def n(inline a: Int)(b: Int = u + a): Int = a * 10 + b
  override def m(a: Int)(b: Int): Int = a * 10 + b + u
  def inside: Int = n(2)() + m(3)()

class S(val s: String) extends AnyVal with T:
  inline def n(b: Int = s.length): Int = b
  override def m(a: Int)(b: Int): Int = a * 10 + b + s.length

extension (v: V) def outside: Int = v.n(1)() + v.m(1)()

object Main:
  def main(args: Array[String]): Unit =
    val v = new V(9)
    println(new V(9).n(2)())
    println(v.m(2)())
    println(v.inside)
    println((v: T).m(2)())
    println(v.outside)
    val s = new S("abc")
    println(s.n())
    println(s.m(4)())
