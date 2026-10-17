package lib

object Util {
  inline def value: Int = 1
  // Scala 2's identifiers, which Scala 3 made keywords.
  val given: Int = 1
  def enum(n: Int): Int = n
}
