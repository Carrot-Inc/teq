class C(val n: Int) { def ==(x: AnyRef): Boolean = true; def ==(x: C): Boolean = true }
def uses(a: C, b: C): Unit =
  println(a == b)
  println(a == "s")
// One alternative of an overloaded own `==` has the type of Any's final `==` after erasure: scalac refuses the
// declaration (E120), whichever alternative a use takes.
// expect: 1:7: error: Name clash between defined and inherited member:
// expect: final def ==(x$0: Any): Boolean in class Any and
// expect: def ==(x: AnyRef): Boolean in class C at line 1
// expect: have the same type (x: Object): Boolean after erasure.
// expect: 1 error found
