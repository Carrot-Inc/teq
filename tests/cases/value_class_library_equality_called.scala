// jars: universal-equality-lib
// A value class below two library traits that override `equals` and `hashCode`, the two also called through the
// class's own type: the later trait's stand, as under scalac 3.8.4.
import unieq.*

class Two(val x: Int) extends AnyVal with Odd with Even

def through(t: Two): (Boolean, Int) = (t.equals(null), t.hashCode)

@main def run(): Unit =
  val a: Any = new Two(1)
  val b: Any = new Two(1)
  println(a == b)
  println(a.hashCode)
  println(through(new Two(3)))
