//> using platform js
@main def run(): Unit =
  val big = 2147483647
  println(big + 1)
  println(big * 2)
  println(-big - 2)
  println(7 / 2)
  println(-7 / 2)
  println(7 % 3)
  println(-7 % 3)
  println(1 << 31)
  println(-16 >> 2)
  println(-16 >>> 28)
  println(5 & 3)
  println(5 | 3)
  println(5 ^ 3)
  println(~5)
  println(46341 * 46341)
  println(123456789 * 987654321)

  val l = 9223372036854775807L
  println(l + 1L)
  println(l / 3L)
  println(-7L / 2L)
  println(-7L % 3L)
  println(1L << 40)
  println(-1L >>> 60)
  println(big.toLong * 2L)
  println(big + 1L)
  println(3000000000L.toInt)
  println(5L == 5)
  println(5L < 6)

  println(1.5 + 2)
  println(10 / 4.0)
  println(1.0 / 3)
  println(2.0 * 3)
  println(1e10)
  println(1.0e-5)
  println(123456789.125)
  println(0.1 + 0.2)
  println(3.7.toInt)
  println(-3.7.toInt)
  println(1e20.toInt)
  println(7.toDouble / 2)
  println(3.99.toLong)
  println(1.0 / 0)
  println(-7.5 % 2)

  val c = 'a'
  println(c)
  println(c + 1)
  println((c + 1).toChar)
  println(c.toInt)
  println('z' - 'a')
  println(c < 'b')
  println('7'.asDigit + 1)
  println(c.toUpper)
  println("" + c + 'b')
  println(s"char $c and ${c.toInt}")

  var i = 10
  i += 5
  i -= 3
  i *= 2
  i /= 5
  i %= 3
  println(i)
  var d = 1.5
  d *= 2
  d += 1
  println(d)
  var s = "a"
  s += "b"
  s += 1
  println(s)

  println(1 + 2 * 3 - 4 / 2)
  println((1 + 2) * 3)
  println(2 + 3 == 5 && 4 > 3 || false)
  println(!(1 < 2))
  println(1 == 1.0)
  println(10.max(3))
  println(3.min(-3))
  println((-5).abs)
  println(2.5.max(1.0))
  println(Int.MaxValue)
  println(Int.MinValue)
  println(Long.MaxValue)
  println(scala.math.sqrt(16.0))
  println(scala.math.max(3, 9))
  println(scala.math.abs(-2.5))
  println(scala.math.pow(2.0, 10.0))
  val mixed = if i > 100 then 1 else 2.5
  println(mixed)
