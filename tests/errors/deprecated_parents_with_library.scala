// jars: scala-library
// expect: 10:23: warning: trait DelayedInit in package scala is deprecated since 2.11.0: DelayedInit semantics can be surprising. Support for `App` will continue. See the release notes for more details: https://github.com/scala/scala/releases/tag/v2.11.0
// expect: 13:22: warning: trait App in package scala is deprecated since 3.8.0: Support for trait App is deprecated in Scala 3. Please refer to https://docs.scala-lang.org/scala3/book/methods-main-methods.html.
// expect: 17:21: warning: trait App in package scala is deprecated since 3.8.0: Support for trait App is deprecated in Scala 3. Please refer to https://docs.scala-lang.org/scala3/book/methods-main-methods.html.
// expect: 20:3: warning: method delayedInit in trait App is deprecated since 2.11.0: the delayedInit mechanism will disappear
// expect: 4 warnings found, errors under --werror
// teq: --werror --deprecation --main Main
// `DelayedInit` and `App` as parents of a trait and of objects, and `App`'s `delayedInit` called: each is warned
// of, also with scala-library's jar on the class path beside the std; a trait's own override of `delayedInit` is not.
trait Journal extends DelayedInit {
  abstract override def delayedInit(x: => Unit): Unit = { println("journal"); super.delayedInit(x) }
}
object Stall extends App with Journal {
  println("stall body")
  delayedInit { println("stall block") }
}
trait Tally extends App { def count: Int = args.length }
object Booth extends Tally {
  println("booth body")
  delayedInit { println("booth " + count) }
}
object Main {
  def main(a: Array[String]): Unit = {
    Stall.main(Array())
    Booth.main(Array("p", "q"))
  }
}
