// A boxed java.lang value takes the operators of its primitive through Predef's unboxing
// conversions, as a receiver and as an operand.
object Main:
  def flag(s: String): java.lang.Boolean = java.lang.Boolean.valueOf(s.nonEmpty)
  def main(args: Array[String]): Unit =
    val i: java.lang.Integer = 1
    val n: Int = i
    println(i + 1)
    println(i * 3 - n)
    println(i < 2)
    println(i.compareTo(2))
    val l: java.lang.Long = 5L
    println(l + i)
    println(-i)
    println(~i)
    val d: java.lang.Double = 2.5
    println((d * 2).toInt)
    val b: java.lang.Boolean = true
    val p: Boolean = b
    println(p)
    println(!b)
    println(b && flag(""))
    println(b || flag(""))
    println(b & flag("x"))
    println(!flag("x"))
    if !b then println("no") else println("yes")
