// expect: main.scala:10:30: error: cannot derive Show for scala.Int
// expect: 1 error found
// A plain inline given is the search's answer as a reference, its macro run after the body is
// typed, as scalac's typer leaves it: the macro's own error at the end of the call it completes
// (scalac 3.8.4 at the same place, "cannot derive Show for scala.Int"). The String instance
// resolves without a word about `derived`.
object Test:
  def main(args: Array[String]): Unit =
    println(summon[Show[String]].show("ok"))
    println(summon[Show[Int]].show(1))
