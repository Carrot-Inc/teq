//> using platform js
//> using dep io.github.cquiroz::scala-java-locales::1.5.4
// interp-expected: js
// java.text.DecimalFormat's scientific notation as a Scala.js build has it from
// scala-java-locales: the mantissa's precision is the minimum integer and maximum fraction
// digits, a maximum integer count above the minimum makes the exponent its multiple
// (engineering notation), and the exponent is written without padding.
import java.text.DecimalFormat

@main def run(): Unit =
  val plain = new DecimalFormat("0.###E0")
  println(List(12345.0, 0.00012345, 0.0, -1234.5, 1.0, 9.9996).map(plain.format).mkString(" "))
  println(plain.format(123456789L) + " " + plain.format(BigDecimal("0.5")))
  val two = new DecimalFormat("00.###E0")
  println(two.format(12345.0) + " " + two.format(0.0012345) + " " + two.format(7L))
  val engineering = new DecimalFormat("##0.#####E0")
  println(List(12345.0, 123456.0, 0.00123, 1234567.89, 1.0).map(engineering.format).mkString(" "))
  println(engineering.getMaximumIntegerDigits + " " + engineering.getMinimumIntegerDigits + " " + engineering.getMaximumFractionDigits)
  val fixed = new DecimalFormat("0.00E0")
  println(fixed.format(1.5) + " " + fixed.format(999.999) + " " + fixed.format(0.1) + " " + fixed.format(-42L))
  val optional = new DecimalFormat("#.##E0")
  println(optional.format(1234.0) + " " + optional.format(0.5))
  val affixed = new DecimalFormat("'x'0.0E0' units'")
  println(affixed.format(31415.9))
