// jars: scala-library nesting-lib
// std: scala-library
// targets: jvm
// A Java class with a static member class `Kind` whose superclass has an inner class `Kind`: a member typed
// `Kind` in a class below it, and in a trait whose self type it is, compiles and runs.
trait Marks { self: nest.Over =>
  def marked: Kind = null
}
class Top extends nest.Over with Marks:
  def k: Kind = null

@main def run(): Unit =
  val t = new Top()
  println(t.k == null)
  println(t.marked == null)
