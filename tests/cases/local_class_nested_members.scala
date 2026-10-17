//> using platform jvm
// split-watch: jvm
// A class nested in a local class keeps its members in a JVM program, called or not, as scalac
// writes every definition of a template: a reflective caller finds the uncalled `bonus`.
object Factory:
  def make: AnyRef =
    class Outer:
      class Inner:
        def bonus: Int = 99
      def value: AnyRef = new Inner
    new Outer().value

@main def run(): Unit =
  println(Factory.make.getClass.getDeclaredMethod("bonus").getName)
