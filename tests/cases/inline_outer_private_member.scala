// An inline method of an inner class reading its outer class's private members names the outer class's `this`, as
// dotty's typer does, which an expansion binds from the receiver through its outer reference
// (`Inliner.computeThisBindings`), and reads them through the outer class's accessors (PrepareInlineable); an ordinary
// default's getter is a method of the inner class, which reads the outer instance it holds; a class or an object the
// body makes reads the outer instance the expansion binds. scalac prints the lines of the .expected file.
class Outer(private val o: Int):
  private def twice(n: Int): Int = n * 2
  val inner = new Inner
  class Inner:
    inline def i: Int = o * 3
    inline def t(n: Int): Int = twice(n) + o
    inline def g(y: Int = o * 3): Int = y
    inline def k(a: Int)(b: Int = a + o): Int = b
    inline def loc: Int = { class L { def v = o + 7 }; L().v }
    inline def obj: Int = { object Lo { val z = o * 10 }; Lo.z }
    class Deeper:
      inline def d: Int = o + 100
object Holder:
  private val secret = 9
  class Nested:
    inline def peek: Int = secret + 1
@main def run(): Unit =
  val o = Outer(2)
  println((o.inner.i, o.inner.t(5), Holder.Nested().peek))
  println((o.inner.g(), o.inner.g(1), o.inner.k(1)()))
  println((o.inner.loc, o.inner.obj))
  val d = new o.inner.Deeper
  println(d.d)
