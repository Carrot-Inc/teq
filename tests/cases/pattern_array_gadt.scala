//> using platform jvm
// A type pattern over an array of a type parameter (`Array[A]` against `Array[Boolean]`) is a test of the
// array's class: the refinement the case brings to `A` makes the scrutinee conform to the pattern only
// inside the case, and `TypeTestsCasts` sees the scrutinee's own type. JavaScript and the interpreter have
// no element kinds (docs/COMPATIBILITY.md): there every array passes, so this runs on the JVM alone.
def direct[A](array: Array[A]): String = array match
  case x: Array[Boolean] => "bool"
  case x: Array[Int] => "int"
  case _ => "other"
def viaCast[A](array: Array[A]): String = (array.asInstanceOf[AnyRef]: @unchecked) match
  case x: Array[Boolean] => "bool"
  case x: Array[Int] => "int"
  case _ => "other"
@main def run(): Unit =
  println(s"${direct(Array(1, 2))},${direct(Array(true))},${direct(Array("s"))}")
  println(s"${viaCast(Array(1, 2))},${viaCast(Array(true))},${viaCast(Array("s"))}")
