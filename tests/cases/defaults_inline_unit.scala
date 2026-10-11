// A `()` written for an inline method's parameter is a value: only the getter's application the
// call's plan makes stands for an omitted argument (`Applications.matchArgs`).
inline def f(a: Unit = print("D")): Unit = a
inline def g(inline a: Unit = print("E"))(b: Int = { print("B"); 1 }): Unit = ()
inline def h(a: Unit = print("H"))(b: Int): Unit = a
@main def main(): Unit = {
  f(()); println("end")
  f(); println("end")
  g(())(); println("end")
  h(())(2); println("end")
}
