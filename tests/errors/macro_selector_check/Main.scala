// expect: Main.scala:5:33: error: read 42, x and 7
// A macro that a check writing nothing runs calls the selectors of the program's case classes,
// one through a trait's abstract member: each selector's body is made where it is first used.
// scalac 3.8.4 reports the same.
@main def run(): Unit = println(M.first)
