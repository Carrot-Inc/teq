// teq: --main Launch
// `main` called before `App`'s own initialiser has run, from the constructor of a superclass: scala-library has
// made no buffer yet and the call fails, where an `App` that is initialised and was given no block returns.
abstract class Early {
  println("early")
  this match { case a: App => a.main(Array.empty[String]); case _ => () }
  println("later")
}
class Job extends Early with App
class Calm extends App {
  println("calm body")
}
object Launch {
  def main(a: Array[String]): Unit = {
    try { new Job; println("made") }
    catch { case _: Throwable => println("no blocks yet") }
    val c = new Calm
    c.main(Array("x"))
    println("ran none")
  }
}
