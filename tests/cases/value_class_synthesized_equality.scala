// A value class equals and hashes as its field does where no base but `Any` defines the two: with no trait, with a
// universal trait that only declares them, and with two traits that each declare `hashCode`, as scalac 3.8.4
// synthesizes both.
class Plain(val x: Int) extends AnyVal

trait Asked extends Any { def hashCode: Int; def equals(o: Any): Boolean }
class Declared(val x: Int) extends AnyVal with Asked

trait Left extends Any { def hashCode: Int }
trait Right extends Any { def hashCode: Int }
class Twice(val x: Int) extends AnyVal with Left with Right

@main def run(): Unit =
  val p1: Any = new Plain(1)
  val p2: Any = new Plain(1)
  val p3: Any = new Plain(2)
  println(p1 == p2)
  println(p1 == p3)
  println(p1.hashCode == 1.hashCode)
  val d1: Any = new Declared(1)
  val d2: Any = new Declared(1)
  val d3: Any = new Declared(2)
  println(d1 == d2)
  println(d1 == d3)
  println(d1.hashCode == 1.hashCode)
  val t1: Any = new Twice(1)
  val t2: Any = new Twice(1)
  println(t1 == t2)
  println(t1.hashCode == 1.hashCode)
