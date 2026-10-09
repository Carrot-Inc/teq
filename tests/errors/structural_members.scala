// expect: 12:11: error: type mismatch: found AnyRef{def m(x: Int): String}, required Selectable | Dynamic
// expect: 13:11: error: type mismatch: found AnyRef{def apply(x: Int): Int}, required Selectable | Dynamic
// expect: 15:18: error: type mismatch: found String, required Int
// expect: 3 errors found
class Sel extends Selectable:
  def selectDynamic(name: String): Any = 1
  def applyDynamic(name: String)(args: Any*): Any = "s"
type Fn = AnyRef { def apply(x: Int): Int }
def plain: AnyRef { def m(x: Int): String } = ???
def fn: Fn = ???
@main def run(): Unit =
  println(plain.m(1))
  println(fn(1))
  val s: Sel { def m(x: Int): String } = ???
  val bad: Int = s.m(1)
