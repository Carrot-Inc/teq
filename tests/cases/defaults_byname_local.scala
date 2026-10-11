// A by-name parameter's default getter returns the value, which the call's thunk calls
// (`Desugar.addDefaultGetters`), for a local def and a local class as for members and
// top-level definitions; a local class's getter takes the class's captures, a var's cell too.
def mark(s: String): Int = { print(s); 1 }
class T(a: => Int = mark("T"))(b: Int) { def value: Int = a + a + b }
object O { def g(a: => Int = mark("G"))(b: Int): Int = a + a + b }
def top(a: => Int = mark("P"))(b: Int): Int = a + a + b
class Sup(a: => Int = mark("S")) { def value: Int = a + a }
class Sub extends Sup
@main def main(): Unit = {
  println(new T()(mark("t")).value)
  println(O.g()(mark("g")))
  println(top()(mark("p")))
  println(new Sub().value)
  var n = 2
  class L(a: => Int = mark("L") + n)(b: Int) { def value: Int = a + a + b }
  println(new L()(mark("l")).value)
  n = 5
  println(new L()(mark("m")).value)
  class K(a: => Int = mark("K"))(b: Int) { def value: Int = a + a + b }
  println(new K()(mark("k")).value)
  def loc(a: => Int = mark("D"))(b: Int): Int = a + a + b
  println(loc()(mark("d")))
}
