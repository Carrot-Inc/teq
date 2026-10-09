// expect: 10:5: error: expected the end of the indented type, found new line
// expect: 2 errors found
// A second type in the region that holds an alias's type is scalac's "unindent expected"; the
// definitions after the alias are kept.
trait A
trait B
object O:
  type T =
    A
    B
  def f: Int = 1
@main def run(): Unit = println(O.f)
