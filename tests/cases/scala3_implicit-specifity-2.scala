// From scala/scala3 tests/run/implicit-specifity-2.scala: Scala 2 implicits ranked by their
// owners and their implicit parameters (`extends App` replaced by a main method).
class Low
object Low {
  implicit val low: Low = new Low
}
class Medium extends Low
object Medium {
  implicit val medium: Medium = new Medium
}
class High extends Medium
object High {
  implicit val high: High = new High
}

class Foo[T](val i: Int)
object Foo {
  def apply[T](implicit fooT: Foo[T]): Int = fooT.i

  implicit def foo[T](implicit priority: Low): Foo[T] = new Foo[T](0)
  implicit def foobar[T](implicit priority: Low): Foo[Bar[T]] = new Foo[Bar[T]](1)
  implicit def foobarbaz(implicit priority: Low): Foo[Bar[Baz]] = new Foo[Bar[Baz]](2)
}
class Bar[T]
object Bar {
  implicit def foobar[T](implicit priority: Medium): Foo[Bar[T]] = new Foo[Bar[T]](3)
  implicit def foobarbaz(implicit priority: Medium): Foo[Bar[Baz]] = new Foo[Bar[Baz]](4)
}
class Baz
object Baz {
  implicit def baz(implicit priority: High): Foo[Bar[Baz]] = new Foo[Bar[Baz]](5)
}

object Test {
  def main(args: Array[String]): Unit = {
    println(Foo[Int])
    println(Foo[Bar[Int]])
    println(Foo[Bar[Baz]])
  }
}
