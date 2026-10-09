//> using platform js
//> using dep io.github.cquiroz::scala-java-locales::1.5.4
// interp-expected: js
// java.text.DecimalFormat as a Scala.js build has it from scala-java-locales: the inherited
// Format.format(Object) beside NumberFormat's format(Double) and format(Long), a Double from its
// shortest decimal form, the pattern's digits, grouping, affixes and negative subpattern.
import java.text.{DecimalFormat, NumberFormat}

@main def run(): Unit =
  val two = new DecimalFormat("0.00")
  println(two.format(BigDecimal("1.5")))
  println(two.format(1.5))
  println(two.format(3L))
  println(two.format(new java.math.BigDecimal("2.345")))
  println(two.format(2.345))
  println(two.format(2.355))
  println(two.format(-0.001))
  println(two.format(Integer.valueOf(7)))
  println(two.format(BigInt(12)))
  val optional = new DecimalFormat("#.##")
  println(optional.format(0.5) + " " + optional.format(12.0) + " " + optional.format(1.005) + " " + optional.format(0))
  val grouped = new DecimalFormat("#,##0.###")
  println(grouped.format(1234567.891) + " " + grouped.format(-1234.5) + " " + grouped.format(999L))
  val wide = new DecimalFormat("0.00######################")
  println(wide.format(BigDecimal("3.14159265358979")) + " " + wide.format(2.0) + " " + wide.format(0.1 + 0.2))
  println(new DecimalFormat("000.0").format(4.25) + " " + new DecimalFormat("0").format(2.5) + " " + new DecimalFormat("0").format(3.5))
  println(new DecimalFormat("#,##0.00;(#,##0.00)").format(-1234.5))
  println(new DecimalFormat("'#'0 'items'").format(42L))
  println(new DecimalFormat("0.0%").format(0.256))
  val f = new DecimalFormat("0.0")
  f.setMinimumFractionDigits(3)
  println(f.format(1.5) + " " + f.getMaximumFractionDigits + " " + f.getMinimumIntegerDigits)
  f.setGroupingUsed(true)
  f.setGroupingSize(3)
  f.setRoundingMode(java.math.RoundingMode.DOWN)
  println(f.format(1234567.98765))
  val nf: NumberFormat = new DecimalFormat("#,##0.##")
  println(nf.format(1234.567) + " " + nf.format(10L) + " " + nf.format(BigDecimal("0.125")))
  try println(two.format("text"))
  catch case e: IllegalArgumentException => println(e.getMessage)
