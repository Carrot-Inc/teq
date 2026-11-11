object Factors:
  inline val wide = 63L
  inline val narrow = 3

object Main:
  def charResult = (Factors.wide * Factors.narrow).toChar
  def intResult = (Factors.wide * Factors.narrow).toInt
  def longResult = (Factors.wide * Factors.narrow).toLong
  def retainedResult =
    val product = Factors.wide * Factors.narrow
    product.toChar

  def main(args: Array[String]): Unit =
    println(charResult.toInt)
    println(intResult)
    println(longResult)
    println(retainedResult.toInt)
