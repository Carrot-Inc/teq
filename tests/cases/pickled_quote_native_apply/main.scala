object Main:
  def integer: Int = 2
  def wide: Long = 3L
  def flag: Boolean = true
  def text: String = "value"
  def main(args: Array[String]): Unit =
    println(NativeShapes.of(integer + integer))
    println(NativeShapes.of(integer + wide))
    println(NativeShapes.of(flag && flag))
    println(NativeShapes.of(flag & flag))
    println(NativeShapes.of(integer == integer))
    println(NativeShapes.of(integer.equals(wide)))
    println(NativeShapes.of(text + integer))
    println(NativeShapes.of(integer + text))
