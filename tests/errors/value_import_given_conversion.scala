// expect: 7:3: error: type mismatch: found Int, required String
// A wildcard from a value leaves its givens out, a `given Conversion` among them.
class C:
  given Conversion[Int, String] = _.toString
def f(c: C): String =
  import c.*
  1
@main def run(): Unit = println(f(C()))
