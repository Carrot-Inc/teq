class C(val n: Int) { def ==(o: AnyRef): Boolean = true }
def uses(a: C): Unit =
  println(a == "s")
// An own `==` whose parameter erases to Object has the type of Any's final `==` after erasure: scalac refuses
// the declaration (E120), used or not.
// expect: 1:7: error: Name clash between defined and inherited member:
// expect: final def ==(x$0: Any): Boolean in class Any and
// expect: def ==(o: AnyRef): Boolean in class C at line 1
// expect: have the same type (o: Object): Boolean after erasure.
// expect: 1 error found
