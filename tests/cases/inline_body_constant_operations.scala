// An inline body's operations over parameters bound to constants, typed again where the body expands (InlineTyper's
// `typedApply`, `ConstFold`) and, of a constant type, their literal (`constToLiteral`): a condition selects its branch
// with nothing left before it, so the expansion is pure for an `inline val`, of the literal type the result asks.
object M:
  transparent inline def pick(x: Int) = if x > 2 then "big" else "small"
  inline def pick2(x: Int): String = if x > 2 then "big" else "small"
  inline val r = pick(3)
  transparent inline def f(x: Int) = x + 1
  inline val v = f(3)
  transparent inline def g(x: Int) = (x * 2) + (x - 1)
  inline val w = g(4)
  transparent inline def neg(b: Boolean) = if !b then 1 else 2
  inline val n = neg(false)
  transparent inline def wide(x: Int) = if x + 2L > 4L then "long" else "short"
  inline val l = wide(3)
  transparent inline def chars(c: Char) = if c + 1 > 98 then 1 else 0
  inline val c = chars('b')
  def main(args: Array[String]): Unit =
    val s: "big" = pick(3)
    val y: 4 = f(3)
    val z: 11 = g(4)
    println(r); println(v); println(w); println(s); println(y); println(z); println(pick2(1))
    println(n); println(l); println(c)
