// jars: scala-library
// scala.math.BigDecimal with a rounding mode: its bodies come from scala-library, and teq stops in
// them (`missing argument for parameter raw` in scala.Enumeration.nmap and vmap, the MathContext
// constructor's overloads; in the corpus's --big-decimal variant also scala.Enumeration.nameOf
// with `too many type arguments` and ArrayBuilder's `value copy of Array` not supported yet), so
// that variant builds under scalac only. scalac prints `1.24 12345`.
object Main:
  def main(args: Array[String]): Unit =
    val a = BigDecimal("1.2345")
    println(a.setScale(2, BigDecimal.RoundingMode.HALF_UP).bigDecimal.toPlainString + " " + (a * BigDecimal(10000)).setScale(0, BigDecimal.RoundingMode.DOWN).toLong)
