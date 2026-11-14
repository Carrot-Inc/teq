// A value class's `equals` and `hashCode` are the rule's only where no base but `Any` defines a matching one, as
// scalac 3.8.4 synthesizes them: a universal trait's stand, one of the two alone too, from two traits up, and for
// every way the two are reached (`==`, `equals`, `hashCode`, `##`, a generic `==`, a set, `contains`).
trait Odd extends Any {
  override def equals(o: Any): Boolean = false
  override def hashCode: Int = 7
}
class Both(val x: Int) extends AnyVal with Odd

trait Hashed extends Any { override def hashCode: Int = 7 }
class OnlyHash(val x: Int) extends AnyVal with Hashed

trait Never extends Any { override def equals(o: Any): Boolean = false }
class OnlyEquals(val x: Int) extends AnyVal with Never

trait Base extends Any {
  override def equals(o: Any): Boolean = false
  override def hashCode: Int = 9
}
trait Mid extends Any with Base
class Far(val x: Int) extends AnyVal with Mid

def same[A](a: A, b: A): Boolean = a == b

@main def run(): Unit =
  val a = new Both(1)
  val b = new Both(1)
  println(a == b)
  println(a.equals(b))
  println(a.hashCode)
  println(a.##)
  println(same(a, b))
  println(Set(a, b).size)
  println(List(a).contains(b))
  val h1: Any = new OnlyHash(1)
  val h2: Any = new OnlyHash(1)
  val h3: Any = new OnlyHash(2)
  println(h1 == h2)
  println(h1 == h3)
  println(h1.hashCode)
  val e1: Any = new OnlyEquals(1)
  val e2: Any = new OnlyEquals(1)
  println(e1 == e2)
  println(e1.hashCode == 1.hashCode)
  val f1: Any = new Far(1)
  val f2: Any = new Far(1)
  println(f1 == f2)
  println(f1.hashCode)
