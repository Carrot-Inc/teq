// jars: scala-library scala2-traits
// A Scala 2.13 jar of traits alone is Scala 2's by its classes' pickles, as dotty's `ClassfileParser` reads each
// class's `ScalaSig` (master took a jar for Scala 2 only with a module class: "type Counter not found"); its
// trait's private var comes from the class file, as a jar with modules' does.
class C extends scala2traits.Counter
class P extends scala2traits.Named { def name = "p" }
@main def run() =
  val c = new C
  println(c.next)
  println(c.next)
  println(new P().greet)
