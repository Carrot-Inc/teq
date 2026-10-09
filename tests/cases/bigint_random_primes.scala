// scala-library's random and prime `BigInt`s over the JDK's algorithms, drawing as the JDK draws
// from the seeded generator, so that a seed gives the JDK's numbers: `BigInt(numbits, rnd)`,
// `BigInt(bitlength, certainty, rnd)` and `probablePrime` below and above the JDK's 95-bit
// threshold; and a `BigDecimal` from an array of characters.
import scala.util.Random

object Main:
  def main(args: Array[String]): Unit =
    val r = new Random(42)
    println((1 to 5).map(_ => BigInt(20, r)).mkString(" "))
    println(BigInt(0, r))
    println(BigInt.probablePrime(8, new Random(0)))
    println(BigInt.probablePrime(31, new Random(7)))
    println(BigInt.probablePrime(64, new Random(1)))
    println(BigInt(40, 20, new Random(3)))
    val big = BigInt.probablePrime(130, new Random(5))
    println(big.bitLength + " " + big.isProbablePrime(50))
    println(big)
    println(BigInt(100, 10, new Random(9)))
    try BigInt.probablePrime(1, r) catch case e: ArithmeticException => println(e.getMessage)
    println(BigDecimal(Array('1', '.', '5', '0')))
    println(BigDecimal(Array('-', '2', 'E', '3')).toBigInt)
