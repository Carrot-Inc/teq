// jars: universal-equality-lib
// A value class below a library's universal traits that override `equals` and `hashCode` (a jar compiled by scalac):
// the traits' stand, one trait's and the later of two, as under scalac 3.8.4.
import unieq.*

class One(val x: Int) extends AnyVal with Odd
class Two(val x: Int) extends AnyVal with Odd with Even

@main def run(): Unit =
  val a: Any = new One(1)
  val b: Any = new One(1)
  println(a == b)
  println(a.hashCode)
  val c: Any = new Two(1)
  val d: Any = new Two(1)
  println(c == d)
  println(c.hashCode)
