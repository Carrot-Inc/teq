// A cast tests the value as `isInstanceOf` does, so the runtime classes a value has under scalac
// pass on every target: the JDK's boxes and their interfaces (`Comparable`, `CharSequence`,
// `java.io.Serializable`) take the primitives and strings, which are JS values on JavaScript; a
// case class, a tuple, a list and an enum case are a `Product` and an `Equals`; a partial
// function literal is a `PartialFunction`.
enum Color:
  case Red, Green
case class Point(x: Int, y: Int)

def attempt(name: String)(f: => Any): Unit =
  try println(name + ": " + f)
  catch case _: ClassCastException => println(name + " CCE")

@main def run(): Unit =
  val seven: Any = 7
  val long: Any = 7L
  val text: Any = "ab"
  attempt("Integer") { seven.asInstanceOf[java.lang.Integer] }
  attempt("Long") { long.asInstanceOf[java.lang.Long] }
  attempt("Boolean") { (true: Any).asInstanceOf[java.lang.Boolean] }
  attempt("Character") { ('c': Any).asInstanceOf[java.lang.Character] }
  attempt("Number") { seven.asInstanceOf[java.lang.Number] }
  attempt("Integer of Long") { long.asInstanceOf[java.lang.Integer] }
  println(seven.isInstanceOf[java.lang.Integer])
  attempt("Comparable of Int") { seven.asInstanceOf[Comparable[?]] }
  attempt("Comparable of String") { text.asInstanceOf[Comparable[?]] }
  attempt("CharSequence") { text.asInstanceOf[CharSequence] }
  attempt("Serializable of String") { text.asInstanceOf[java.io.Serializable] }
  attempt("Product of a case class") { (Point(1, 2): Any).asInstanceOf[Product].productArity }
  attempt("Product of a tuple") { ((1, "a"): Any).asInstanceOf[Product].productArity }
  attempt("Product of a list") { (List(1): Any).asInstanceOf[Product].productArity }
  attempt("Equals of a case class") { (Point(1, 2): Any).asInstanceOf[Equals] }
  attempt("Product of an enum case") { (Color.Red: Any).asInstanceOf[Product].productPrefix }
  println((Point(1, 2): Any).isInstanceOf[Product])
  val pf: Any = ({ case 1 => 2 }: PartialFunction[Int, Int])
  println(pf.isInstanceOf[PartialFunction[?, ?]])
  attempt("PartialFunction") { pf.asInstanceOf[PartialFunction[Int, Int]](1) }
