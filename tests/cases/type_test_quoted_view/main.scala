// What a macro sees of a type test and of a cast: `x.isInstanceOf[T]` with `T` the type the source wrote, once,
// whatever test the back ends make of it (`Any` and `AnyVal` are both the test that the value is not `null`), and
// a quote pattern over a type test matches the tested type as a typed pattern's (a test of `Int` is one of `AnyVal`).
// teq showed the test's own type, twice (`x.isInstanceOf[scala.AnyVal][scala.AnyVal]` for a test of `Any`), and
// matched by the test.
@main def run(): Unit =
  val x: Any = 5
  val xs: Seq[Int] = List(1)
  val o: Option[String] = Some("s")
  println(Show.show(x.isInstanceOf[Any]))
  println(Show.show(x.isInstanceOf[AnyVal]))
  println(Show.show(x.isInstanceOf[AnyRef]))
  println(Show.show(x.isInstanceOf[String]))
  println(Show.show(x.isInstanceOf[Int]))
  println(Show.show(xs.isInstanceOf[collection.immutable.ArraySeq[Int]]))
  println(Show.show(o.isInstanceOf[Some[String]]))
  println(Show.show(x.asInstanceOf[String]))
  println(Show.show(xs.asInstanceOf[collection.immutable.ArraySeq[Int]]))
  println(Show.kind(x.isInstanceOf[Any]))
  println(Show.kind(x.isInstanceOf[AnyVal]))
  println(Show.kind(x.isInstanceOf[Int]))
  println(Show.kind(x.isInstanceOf[String]))
