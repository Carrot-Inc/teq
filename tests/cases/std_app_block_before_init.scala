// teq: --main Launch
// `delayedInit` called before `App`'s own initialiser has run, from the constructor of a superclass:
// scala-library has no buffer to keep the block in yet and the call fails, where one given after it is kept.
abstract class Early {
  println("early")
  this match { case a: App => a.delayedInit { println("block") }; case _ => () }
  println("later")
}
class Job extends Early with App {
  println("body")
}
class Calm extends App {
  println("calm body")
}
object Launch {
  def main(a: Array[String]): Unit = {
    try { val j = new Job; println("made"); j.main(Array("x")) }
    catch { case _: Throwable => println("no blocks yet") }
    val c = new Calm
    c.delayedInit { println("kept one") }
    c.delayedInit { println("kept two") }
    c.main(Array("x"))
  }
}
