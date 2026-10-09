// `initCause` gives an exception its cause after construction, once: a second cause, or the
// exception itself, is refused (utest's `AssertionError` passes its cause so). And
// `ClassNotFoundException` with its cause, and `InstantiationError`.
class Failed(msg: String, cause: Throwable) extends Exception(msg):
  initCause(cause)

@main def run(): Unit =
  val root = new IllegalArgumentException("root")
  val e = new Failed("outer", root)
  println(e.getCause eq root)
  try e.initCause(new RuntimeException("again"))
  catch case x: IllegalStateException => println("refused: " + x.getMessage.takeWhile(_ != '('))
  val self = new RuntimeException("self")
  try self.initCause(self)
  catch case x: IllegalArgumentException => println("refused: " + x.getMessage)
  val plain = new RuntimeException("plain")
  println(plain.initCause(null) eq plain)
  println(plain.getCause)
  val missing = new ClassNotFoundException("a.B", root)
  println(missing.getMessage + " " + (missing.getCause eq root) + " " + (missing.getException() eq root))
  println(new InstantiationError("gone").getMessage + " " + new InstantiationError("x").isInstanceOf[LinkageError])
