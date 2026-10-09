// `codeOf` and `requireConst` over the walked argument, as scalac's `Inlines` shows it
// (`arg.show`): a constant folded to its literal, an argument an enclosing expansion substituted
// shown with what stands in it (`y + 1` from `code(x + 1)` with `x` for `y`), an operation infix,
// a unary one as its method. The retype path shows the parameter's name. scalac prints the lines
// of the .expected file.
import scala.compiletime.{codeOf, requireConst}
inline def code(inline x: Int): String = codeOf(x)
inline def code2(inline x: Int): String = code(x + 1)
inline def codeS(inline s: String): String = codeOf(s)
inline def codeB(inline b: Boolean): String = codeOf(b)
inline def codeAny(inline a: Any): String = codeOf(a)
inline def need(inline x: Int): Int = { requireConst(x); x }
def f(i: Int): Int = i
@main def run(): Unit =
  val y = 5
  println(code(1 + 2))
  println(code2(3))
  println(code(y * 2))
  println(code(if y > 1 then y else 0))
  println(code2(y))
  println(codeS("s"))
  println(codeS("a" + "b"))
  println(codeB(y > 3))
  println(code(f(y)))
  println(code(10L.toInt))
  println(codeAny('c'))
  println(codeAny(2.5))
  println(code(-y))
  println(need(1 + 2))
  println(need(40 + 2))
