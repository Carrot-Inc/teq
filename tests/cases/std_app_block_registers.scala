// teq: --main Launch
// A block that gives `delayedInit` another block while `main` runs them: scala-library keeps the blocks in a
// buffer and runs them by a `for`, which a mutation during the iteration ends with an exception.
object Test extends App {
  println("body")
  delayedInit { println("first"); delayedInit { println("never run") } }
  delayedInit { println("second") }
}
object Launch {
  def main(a: Array[String]): Unit =
    try Test.main(a)
    catch { case e: java.util.ConcurrentModificationException => println("refused: " + e.getMessage) }
}
