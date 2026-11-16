// teq: --main Launch
// The blocks a program gives `delayedInit` itself: `main` runs them in the order given, at each call, after it
// set the arguments. The object's body ran once, when the object was first touched, with `args` still null.
object Test extends App {
  println("body, args null: " + (args == null))
  delayedInit { println("first block, " + args.length + " arguments") }
  delayedInit { println("second block") }
  def count: Int = args.length
}
object Launch {
  def main(a: Array[String]): Unit = {
    println("before")
    Test.main(Array("x"))
    println("between " + Test.count)
    Test.main(Array("y", "z"))
    println("after " + Test.count)
  }
}
