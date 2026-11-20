// An integer literal whose digits the class expected does not hold is refused at the literal, as scalac's
// `FromDigits` refuses it: a `Long` past its range, an `Int`'s digits past its range, a separator at the digits' end.
// expect: 8:17: error: number too large
// expect: 9:16: error: number too large
// expect: 10:17: error: number too large
// expect: 11:17: error: trailing separator is not allowed
object Main:
  val a: Long = 9223372036854775808
  val b: Int = 4294967295
  val c: Long = -9223372036854775809
  val d: Int = 1_
  def main(args: Array[String]): Unit = println((a, b, c, d))
