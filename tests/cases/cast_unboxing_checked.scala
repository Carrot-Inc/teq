// `x.asInstanceOf[Int]` of a reference unboxes as `BoxesRunTime.unboxToInt` does: a value of
// another kind fails with `ClassCastException`, discarded too, on every target that tells the
// kinds apart (a `Long` is a `BigInt` on JavaScript, a `Boolean` a boolean); a primitive cast to
// a primitive converts.
def attempt(name: String)(f: => Any): Unit =
  try println(name + ": " + f)
  catch case e: ClassCastException => println(name + ": " + e.getMessage.takeWhile(_ != '(').trim)

@main def run(): Unit =
  println(257.asInstanceOf[Byte])
  println(7.asInstanceOf[Long] + 1L)
  println('a'.asInstanceOf[Int])
  println(97.asInstanceOf[Char])
  println(7.9.asInstanceOf[Int])
  val seven: Any = 7
  val long: Any = 7L
  val text: Any = "wrong"
  val yes: Any = true
  attempt("Int as Int") { seven.asInstanceOf[Int] + 1 }
  attempt("Int as Long") { seven.asInstanceOf[Long] + 1L }
  attempt("Long as Int") { long.asInstanceOf[Int] + 1 }
  attempt("String as Int") { text.asInstanceOf[Int] + 1 }
  attempt("String as Int, discarded") { text.asInstanceOf[Int]; "done" }
  attempt("String as Boolean") { !text.asInstanceOf[Boolean] }
  attempt("Boolean as Int") { yes.asInstanceOf[Int] }
  attempt("Boolean as Boolean") { !yes.asInstanceOf[Boolean] }
