//> using platform js
// `BigDecimal.decimal` of a `Float` takes the `Float`'s own decimal text (scala-library's `decimal(f: Float)`,
// `java.lang.Float.toString`), where `BigDecimal(f)` widens it to a `Double` first; an `Int` or a `Long` still
// takes the `Long` overload. On JavaScript a `Float` prints as the `Double` of its value, as under Scala.js
// (tests/jvm-expected has the JVM's answers).
import java.math.MathContext

object Main:
  def main(args: Array[String]): Unit =
    println(BigDecimal(0.1f))
    println(BigDecimal.decimal(0.1f))
    println(BigDecimal.decimal(0.1f, new MathContext(1)))
    println(BigDecimal.decimal(3.4028235e38f))
    println(BigDecimal.decimal(1.0e-10f))
    println(BigDecimal.decimal(1))
    println(BigDecimal.decimal(7L))
    println(BigDecimal.decimal(0.1))
    println(BigDecimal(Array('1', '.', '2')))
    println(BigDecimal.decimal(0.1f) == BigDecimal(0.1))
