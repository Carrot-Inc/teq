// An array of `Nothing` is an array, no bottom: its cast to a class, a trait or a primitive is
// checked and fails, its cast to an array, `Any` or `AnyRef` conforms as its element does.
class B
trait Tag
def attempt(label: String)(f: => Any): Unit =
  try { f; println(label + " ok") }
  catch case _: ClassCastException => println(label + " CCE")
@main def run(): Unit =
  val xs = new Array[Nothing](0)
  attempt("to class") { xs.asInstanceOf[B]; () }
  attempt("to trait") { xs.asInstanceOf[Tag]; () }
  attempt("to Int") { xs.asInstanceOf[Int]; () }
  val ys = new Array[Array[Nothing]](0)
  attempt("nested to class") { ys.asInstanceOf[B]; () }
  attempt("to array") { xs.asInstanceOf[Array[B]]; () }
  attempt("nested to array") { ys.asInstanceOf[Array[Array[B]]]; () }
  attempt("to Any") { println(xs.asInstanceOf[Any] != null) }
  attempt("to AnyRef") { println(xs.asInstanceOf[AnyRef] != null) }
  val zs = new Array[Null](1)
  attempt("nulls to strings") { zs.asInstanceOf[Array[String]]; () }
