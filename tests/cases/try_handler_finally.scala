// The finaliser of a `try` runs after every exit of its catch cases as after its protected
// expression's: a guard that returns, a guard that throws, a pattern whose extractor throws and a
// case body that returns, each called often enough to run as the interpreter's lowered code too
// (dotty protects the handlers with the finaliser, BCodeSyncAndTry.genLoadTry).
object Boom:
  def unapply(e: Throwable): Option[String] = throw new IllegalStateException("extractor")

object Main:
  def guardReturns(): Int =
    try throw new RuntimeException("initial")
    catch case _: RuntimeException if { return 7; false } => 0
    finally println("guard-finally")

  def guardThrows(): Int =
    try throw new RuntimeException("initial")
    catch case _: RuntimeException if { throw new IllegalArgumentException("guard"); false } => 0
    finally println("throw-finally")

  def extractorThrows(): Int =
    try throw new RuntimeException("initial")
    catch case Boom(m) => m.length
    finally println("extractor-finally")

  def bodyReturns(n: Int): Int =
    try throw new RuntimeException("initial")
    catch case _: RuntimeException => return n * 2
    finally println("body-finally")

  def main(args: Array[String]): Unit =
    for i <- 1 to 3 do
      println(guardReturns())
      try println(guardThrows()) catch case e: IllegalArgumentException => println(e.getMessage)
      try println(extractorThrows()) catch case e: IllegalStateException => println(e.getMessage)
      println(bodyReturns(i))
