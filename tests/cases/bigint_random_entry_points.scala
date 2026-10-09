// The random and prime `BigInt`s go through the JDK's `BigInteger` entry points
// (`BigInteger(numBits, rnd)`, `BigInteger(bitLength, certainty, rnd)`, `BigInteger.probablePrime`)
// over the `java.util.Random` that `scala.util.Random` wraps, as scala-library's `BigInt` does, so a
// seed gives the JVM's numbers; the generator is the JDK's, drawn on directly too.
import java.math.BigInteger

@main def run(): Unit =
  println(BigInt(9, new scala.util.Random(99L)))
  println(BigInt.probablePrime(10, new scala.util.Random(99L)))
  println(BigInt(10, 5, new scala.util.Random(99L)))
  println(BigDecimal(Array('3', '.', '1', '4'), new java.math.MathContext(2)))
  println(new BigInteger(70, new java.util.Random(5L)))
  println(new BigInteger(20, 10, new java.util.Random(5L)))
  println(BigInteger.probablePrime(40, new java.util.Random(5L)))
  val r = new java.util.Random(42L)
  println(s"${r.nextInt()} ${r.nextInt(10)} ${r.nextInt(16)} ${r.nextLong()} ${r.nextBoolean()}")
  println(s"${r.nextDouble()} ${(r.nextFloat() * (1 << 24)).toInt}")
  val bytes = new Array[Byte](6)
  r.nextBytes(bytes)
  println(bytes.mkString(","))
  r.setSeed(42L)
  println(r.nextInt())
  val s = new scala.util.Random(7)
  println(s"${s.nextInt(100)} ${s.self.nextInt(100)} ${s.between(-5, 5)} ${s.nextLong(1000000000000L)} ${s.nextPrintableChar()}")
  println(s.shuffle(List(1, 2, 3, 4, 5)))
