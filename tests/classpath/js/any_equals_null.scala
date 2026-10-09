// targets: jvm interp
// `x.equals(y)` on a null receiver typed `Any` throws NullPointerException on the JVM and in
// the interpreter, where `==` answers, once the argument is evaluated. (JavaScript fails as any
// member read of null does, and Scala.js's default is undefined behaviour.)
@main def run(): Unit =
  val nil: Any = null
  var evaluated = false
  def arg(): Any = { evaluated = true; null }
  try println(nil.equals(arg()))
  catch case _: NullPointerException => println("null receiver")
  println(evaluated)
  println(nil == null)
