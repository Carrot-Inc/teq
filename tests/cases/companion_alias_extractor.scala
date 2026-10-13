// A val of a companion's type stands for the companion in a pattern (`FooAlias(_)` is
// `FooAlias.unapply`, the synthesized one of `Foo`): scala/pos/t7377b. A typed pattern of an opaque
// type is tested at its alias's class, as `ElimOpaque` erases both sides before `TypeTestsCasts`
// judges the test (scala/pos/i18364.Tup).
object Test:
  case class Foo(a: Int)
  val FooAlias: Foo.type = Foo
  case class Bar()
  val BarAlias: Bar.type = Bar

  def fromIArray[T](xs: IArray[T]): Int =
    val xs2: IArray[Object] = xs match
      case xs: IArray[Object] @unchecked => xs
      case _ => xs.map(_.asInstanceOf[Object])
    xs2.length

@main def run(): Unit =
  import Test.*
  println(Foo(3) match { case FooAlias(n) => n })
  println(Bar() match { case BarAlias() => "bar" })
  println(List(1) match { case Nil => 0; case x :: xs => x })
  println(fromIArray(IArray("a", "b")))
