// Every ordering comparison with a NaN operand is false and `!=` true, in a condition as in a
// value, for Double and Float, whichever side the NaN is on.
object Main:
  def row(label: String, x: Double, y: Double): Unit =
    val values = List(x < y, x <= y, x > y, x >= y, x == y, x != y)
    val conds = List(if x < y then 1 else 0, if x <= y then 1 else 0, if x > y then 1 else 0,
      if x >= y then 1 else 0, if x == y then 1 else 0, if x != y then 1 else 0)
    val negated = List(if !(x < y) then 1 else 0, if !(x <= y) then 1 else 0, if !(x > y) then 1 else 0,
      if !(x >= y) then 1 else 0)
    println(label + " " + values.mkString(",") + " " + conds.mkString + " " + negated.mkString)
  def rowf(label: String, x: Float, y: Float): Unit =
    println(label + " " + List(x < y, x <= y, x > y, x >= y, x == y, x != y).mkString(",") + " " +
      List(if x < y then 1 else 0, if x <= y then 1 else 0, if x > y then 1 else 0, if x >= y then 1 else 0).mkString)
  def main(args: Array[String]): Unit =
    val nan = Double.NaN
    row("1 nan", 1.0, nan)
    row("nan 1", nan, 1.0)
    row("nan nan", nan, nan)
    row("1 2", 1.0, 2.0)
    row("2 2", 2.0, 2.0)
    row("-0 0", -0.0, 0.0)
    rowf("f 1 nan", 1.0f, Float.NaN)
    rowf("f nan 1", Float.NaN, 1.0f)
    rowf("f 1 2", 1.0f, 2.0f)
    var n = 0
    while n < 3 && !(nan > n) do n += 1
    println(n)
    println(List(3.0, nan, 1.0).count(_ > 2.0))
