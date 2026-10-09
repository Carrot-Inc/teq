// A numeric receiver widens to the receiver type of an extension as an argument would, once
// no extension takes it as it is.

object U:
  extension (instance: Long) def showLong: String = "L" + instance.toString
  extension (d: Double) def half: Double = d / 2
  extension (i: Int) def twice: Int = i * 2
  extension (l: Long) def twice: Long = l * 3
@main def main(): Unit =
  import U.*
  val i: Int = 5
  println(i.showLong)
  println(7.showLong)
  println(i.half)
  println(i.twice)
  println(3L.twice)
  println('a'.showLong)
  println((-1).toHexString)
  println(4.toHexString)
  println(4L.toHexString)
