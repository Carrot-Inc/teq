// jars: scala-library
// A lazy val holding scala-library's `LazyVals.NullValue` itself, the mark of a computed null in scalac's
// holders: a local one keeps it (scalac's `LazyRef` keeps `initialized` apart from the value; teq's cell marks a
// computed null with itself), a member's answers it at the read that computes it and null after, as scalac's
// holder does.
class M:
  val marker: AnyRef = scala.runtime.LazyVals.NullValue
  lazy val held: AnyRef = marker
  var computed = 0
  lazy val none: AnyRef = { computed += 1; null }

@main def run(): Unit =
  val marker: AnyRef = scala.runtime.LazyVals.NullValue
  var computed = 0
  lazy val local: AnyRef = { computed += 1; marker }
  lazy val none: AnyRef = { computed += 1; null }
  println(List(local eq marker, local eq marker, none == null, none == null, computed))
  val m = new M
  println(List(m.held eq m.marker, m.held eq m.marker, m.none == null, m.none == null, m.computed))
