// scalac's `ConstantValue` of an argument an inline parameter stands for: a call whose result
// type is a constant (`cond(): true`, `one(): 1`, `oneOf(0)`, `same(3)` of `x.type`) is one, for an
// `if`, an `inline if` and `requireConst` alike. scalac prints the lines of the .expected file.
import scala.compiletime.{error, requireConst}
def cond(): true = true
def one(): 1 = 1
def oneOf(n: Int): 1 = 1
def same(x: Int): x.type = x
def yes(n: Int): true = true
inline def f(inline b: Boolean): Int = if b then 1 else error("dead")
inline def g(inline b: Boolean): Int = inline if b then 2 else error("dead")
inline def need(inline x: Int): Int = { requireConst(x); x }
@main def run(): Unit =
  println(f(cond()))
  println(g(cond()))
  println(need(one()))
  println(need(oneOf(0)) + need(same(3)))
  println(f(yes(2)))
