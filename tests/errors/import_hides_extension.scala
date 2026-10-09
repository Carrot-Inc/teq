// expect: 10:3: error: value a is not a member of Int
// expect: 13:3: error: value a is not a member of Int
// A selector `a as _` hides the extension from the wildcard beside it, of an object and of a value.
class C:
  extension (i: Int) def a: Int = i + 1
object O:
  extension (i: Int) def a: Int = i + 2
def f(c: C): Int =
  import c.{a as _, *}
  0.a
def g: Int =
  import O.{a as _, *}
  0.a
@main def run(): Unit = println(f(C()) + g)
