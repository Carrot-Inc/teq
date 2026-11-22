// A sealed Java interface read from its source: its subclasses among the build's Java sources are its cases, and a
// Scala class may extend it from another file (scalac's reading of a Java sealed type).
class X extends C
object Main {
  def f(s: S): Int = s match { case _: A => 1; case _: B => 2; case _: C => 3 }
  def main(args: Array[String]): Unit = println(f(new B) + " " + f(new X))
}
