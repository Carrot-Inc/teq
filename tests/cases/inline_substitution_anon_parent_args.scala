// The arguments an anonymous class of an inline body passes its parent, copied once for each
// expansion and evaluated where the class is created: a kept inline call among them expanded
// there (`inc(x)`), beside a captured val and the receiver's member. scalac prints the lines of
// the .expected file.
class Base(val n: Int)
inline def inc(x: Int): Int = x + 1
inline def make(x: Int): Base =
  new Base(inc(x)) { def extra: Int = 0 }
inline def twice(x: Int): Int =
  val k = x * 2
  new Base(inc(k) + inc(x)) { override def toString = s"base $k" }.n + make(x).n
class Holder(val m: Int):
  inline def made: Base = new Base(inc(m)) { def tag = m }
@main def run(): Unit =
  println(make(10).n)
  println(twice(3))
  println(Holder(5).made.n)
