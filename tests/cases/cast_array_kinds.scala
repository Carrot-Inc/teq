//> using platform jvm
// An array cast is checked as its erasures relate (`TypeComparer`'s `JavaArrayType`): a primitive
// array is no array of references, nested or not, an array of `Nothing` (a `Nothing$[]`) is no
// primitive array, and an array of `Int` none of `Unit`. JavaScript and the interpreter have no
// element kinds (docs/COMPATIBILITY.md): there every array passes, so this runs on the JVM alone.
def attempt(label: String)(f: => Any): Unit =
  try { f; println(label + " ok") }
  catch case _: ClassCastException => println(label + " CCE")
@main def run(): Unit =
  val a = Array(1, 2)
  attempt("primitive-object-array") { a.asInstanceOf[Array[AnyRef]]; () }
  attempt("nested-primitive-object-array") { Array(a).asInstanceOf[Array[Array[AnyRef]]]; () }
  attempt("nothing-scalar") { new Array[Nothing](0).asInstanceOf[String]; () }
  attempt("nothing-primitive-array") { new Array[Nothing](0).asInstanceOf[Array[Int]]; () }
  attempt("unit-array") { (Array(1): Any).asInstanceOf[Array[Unit]]; () }
  attempt("same kind") { a.asInstanceOf[Array[Int]]; () }
  attempt("strings as objects") { Array("s").asInstanceOf[Array[AnyRef]]; () }
  attempt("primitive as object") { a.asInstanceOf[AnyRef]; () }
