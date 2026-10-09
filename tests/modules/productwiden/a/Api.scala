// Implicit widenings read back from a module's products are the nodes a
// build from source has (`Int.int2double(i)` is not a call), so the whole and the product-based bundles agree.
package widenapi
case class Box(d: Double, s: Short, i: Int)
object Api:
  inline def wide(x: Int): Double = x.toDouble
  inline def short(x: Byte): Short = x.toShort
  inline def integer(x: Short): Int = x.toInt
  def default(d: Double = List(3).head): Double = d
  def tuple(i: Int, b: Byte, s: Short): (Double, Short, Int) = (i, b, s)
  def box(i: Int, b: Byte, s: Short): Box = Box(i, b, s)
  def wideChar(c: Char): Double = c
  def floatChar(c: Char): Float = c
  def rounded(i: Int): Double = (i: Float)
