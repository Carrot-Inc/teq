// scala-library's `Predef` has two `println`s, the
// one of no arguments a Scala 2 method that a bare
// `println` calls: an empty line on every target.
@main def run(): Unit =
  println("a")
  println
  println("b")
  if "b".nonEmpty then
    println
  val f: Any => Unit = println
  f("c")
  val g: () => Unit = println
  g()
  println("d")
