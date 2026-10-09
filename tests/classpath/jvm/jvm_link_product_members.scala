// jars: scala-library
// std: scala-library
// Product members scala-library's code calls on the program's products: canEqual on case classes,
// case objects and enum cases, productElementName through productElementNames.
enum Shape:
  case Circle(radius: Double)
  case Square
case object Origin
case class Pair[A](left: A, right: A)
def show(p: Product): String =
  val names = p.productElementNames.mkString(",")
  s"${p.productPrefix} [$names] ${p.canEqual(p)} ${p.canEqual("x")}"
@main def run(): Unit =
  println(show(Shape.Circle(1.5)))
  println(show(Shape.Square))
  println(show(Origin))
  println(show(Pair(1, 2)))
  println(show((1, "a")))
  println(Pair(1, 2).productElementName(1))
  try Pair(1, 2).productElementName(2)
  catch case e: IndexOutOfBoundsException => println("out of bounds")
