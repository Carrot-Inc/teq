//> using platform js
// interp-expected: js
// A given alias of an object whose initialiser reads itself: Scala.js makes it as a lazy val
// without a latch and recurses, and the interpreter's evaluation of the initialiser is a level of
// its own (`enter`), so its recursion ends in the bound's `StackOverflowError` as a module's does
// (the JVM's thread waits on its own lazy latch, dotty's `LazyVals`, and never ends: no JVM case).
trait T
object O:
  given x: T =
    println(x == null)
    new T {}
@main def run(): Unit =
  val r = try { if O.x != null then "instance" else "null" } catch case e: Throwable => "overflow"
  println(r)
