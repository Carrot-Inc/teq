// An inline method's ordinary getters are methods like any other (`Desugar.addDefaultGetters`):
// an earlier clause's default is lifted before a later clause's arguments as for any call
// (`Lifter.liftArgs`, `wouldLift` exempt only by-name and inline parameters), and the defaults of
// one clause come after its named arguments.
def mark(s: String): Int = { print(s); 1 }
inline def f(a: Int = mark("A"))(b: Int, c: Int = mark("C")): Int = a + b + c
inline def g(a: Int = mark("A"), b: Int = mark("B"), c: Int = mark("C")): Int = a + b + c
inline def h(inline a: Int = mark("I"))(b: Int = a + mark("J")): Int = a + a + b
object Obj { inline def k(a: Int = mark("K"))(b: Int = mark("L")): Int = a + b }
// An ordinary getter whose argument is an inline parameter's getter, lifted by a later clause.
inline def m(inline a: Int = mark("A"))(b: Int = a + mark("B"))(c: Int = mark("C")): Int = a + b + c
inline def n(inline a: Int = mark("A"))(b: Int = a + mark("B"))(c: Int, d: Int = mark("D")): Int = a + b + c + d
@main def main(): Unit = {
  println(f()(mark("B")))
  println(f()(c = mark("c"), b = mark("b")))
  println(g(c = mark("c"), b = mark("b")))
  println(g(b = mark("b")))
  println(h()())
  println(Obj.k()())
  println(m()()())
  println(n()()(mark("c")))
}
