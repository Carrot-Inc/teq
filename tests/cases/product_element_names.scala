// A product's element names: a case class's fields (a tuple's `_1`...), none for a case object
// and an enum value, an index out of range refused; a product of the program with empty names
// or with its own (zio-test, munit and utest print a case class by its fields' names).
final case class Foo(a: Int, `b c`: String)
case object Obj
enum E:
  case Leaf(x: Int)
  case Empty
class Plain extends Product:
  def productArity = 2
  def productElement(n: Int): Any = n
  def canEqual(that: Any) = true
class Named extends Product:
  def productArity = 1
  def productElement(n: Int): Any = n
  def canEqual(that: Any) = true
  override def productElementName(n: Int): String = "only"

@main def run(): Unit =
  val f = Foo(1, "x")
  println(f.productElementNames.toList)
  println((1, "two", 3.0).productElementNames.toList)
  println(Obj.productElementNames.toList)
  println(E.Leaf(3).productElementNames.toList)
  println(E.Empty.productElementNames.toList)
  println(new Plain().productElementNames.toList)
  println(new Named().productElementNames.toList)
  try f.productElementName(5) catch case e: IndexOutOfBoundsException => println("out: " + e.getMessage)
  try new Plain().productElementName(2) catch case e: IndexOutOfBoundsException => println("out: " + e.getMessage)
  val p: Product = f
  println(p.productElementName(1))
  println(p.productElementNames.zip(p.productIterator).toList)
