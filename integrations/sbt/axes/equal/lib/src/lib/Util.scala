package lib

object Util {
  inline def twice(x: Int): Int = x * 2
  def greeting: String = s"${Generated.word}, ${twice(21)}"
}
