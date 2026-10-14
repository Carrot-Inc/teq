// scala-library's `NumericRange` over an `Integral`: `BigInt`'s `to` and `until` (a step of one unless given),
// `BigDecimal`'s, which without a step are a `Range.Partial` that `by` completes, and `Long`'s and `Char`'s;
// a length counted through an `Int` range where the bounds fit and in the type itself where they do not,
// failing on a zero step and past `Int.MaxValue` elements, as scala-library's `NumericRange.count` does.
object Main:
  def main(args: Array[String]): Unit =
    println((BigInt(1) to BigInt(3)).toList)
    println((BigInt(1) until BigInt(10) by BigInt(4)).toList)
    println(BigInt(1).to(BigInt(9), BigInt(3)))
    println((BigDecimal(1) to BigDecimal(2) by BigDecimal("0.5")).toList)
    println((BigDecimal(1) until BigDecimal(2) by BigDecimal("0.25")).length)
    println(BigDecimal(1) to BigDecimal(2))
    println((1L to 3L).toList)
    println(1L to 10L by 3L)
    println(('a' to 'e' by 2).toList)
    println((BigInt("100000000000000000000") until BigInt("100000000000000000010")).length)
    println((0L to Long.MaxValue by (Long.MaxValue / 4)).toList)
    try println((1L to 10L by 0L).length) catch case e: IllegalArgumentException => println(e.getMessage)
    try println((0L to Long.MaxValue).length) catch case e: IllegalArgumentException => println(e.getMessage)
    println((BigInt(5) to BigInt(1) by BigInt(-2)).toList)
    println((BigInt(-3) to BigInt(3) by BigInt(2)).toList)
    println((BigInt(1) to BigInt(3)).map(_ * 2))
    println(List((1L until 1L).isEmpty, (BigInt(3) until BigInt(3)).isEmpty, (BigInt(1) to BigInt(3)).last))
    println((BigInt(1) to BigInt(5)).filter(_ % 2 == 0).sum)
