// A program's own local named and typed as the compiler's enclosing instance (`$thisFake: O.this.type`, null), captured
// by a local class whose superclass's constructor reads it through an override: a capture like any other, stored
// before the superclass call without the outer's null check (only the instance dotty's `ExplicitOuter` makes the
// class's `$outer` is checked).
abstract class A { def f: String; println("A=" + f) }
class O {
  def run(): Unit = {
    val `$thisFake`: O.this.type = null
    class L extends A { def f: String = if `$thisFake` == null then "null" else "nonnull"; println("L-body") }
    new L
  }
}
object Main { def main(args: Array[String]): Unit = new O().run() }
