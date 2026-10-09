// The JDK's `Throwable(message, cause, enableSuppression, writableStackTrace)`: with suppression
// disabled `addSuppressed` still refuses the exception itself and `null`, then records nothing,
// and `getSuppressed` is empty; enabled, it answers a copy of what was added, in order.
class Quiet extends Throwable("quiet", null, false, false)
class Loud extends Exception("loud", null, true, false)
class QuietRuntime extends RuntimeException("q", null, false, true)

@main def run(): Unit =
  val q = new Quiet
  q.addSuppressed(new RuntimeException("lost"))
  println(q.getSuppressed().length)
  try q.addSuppressed(q) catch case _: IllegalArgumentException => println("self")
  try q.addSuppressed(null) catch case _: NullPointerException => println("null")
  val r = new QuietRuntime
  r.addSuppressed(new Error("e"))
  println(r.getSuppressed().length)
  val l = new Loud
  l.addSuppressed(new RuntimeException("a"))
  l.addSuppressed(new RuntimeException("b"))
  val got = l.getSuppressed()
  println(got.map(_.getMessage).mkString(","))
  got(0) = l
  val again = l.getSuppressed()
  println(again(0).getMessage)
