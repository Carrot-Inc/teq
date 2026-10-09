// The second code pass's `random_subclass` program.
class Bytes extends java.util.Random(17L):
  override def nextBytes(a: Array[Byte]): Unit =
    var i = 0
    while i < a.length do
      a(i) = 42.toByte
      i += 1
@main def run(): Unit =
  val r = new Bytes
  println(new java.math.BigInteger(17, r))
  println(BigInt(17, new scala.util.Random(r)))
  val g = new java.util.Random(77L)
  val xs = new Array[Byte](7)
  g.nextBytes(xs)
  println(xs.mkString(","))
  println(g.nextInt(1073741825))
  println(g.nextLong())
  println(BigInt.probablePrime(16, new scala.util.Random(73L)))
  println(new java.math.BigDecimal(Array('2','.','5','5'), new java.math.MathContext(2)))
