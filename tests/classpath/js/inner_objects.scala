// jars: fixtures
// Objects nested in a class of a jar are per instance, a companion of a nested trait too: their
// methods and the anonymous classes they make read the outer instance's members, and one is an
// extractor that calls itself.
import fix.cake.Outer

@main def main(): Unit =
  val o = new Outer("!")
  println(o.run("a"))
  println(new Outer("?").run("b"))
  println(o.peel("((x"))
