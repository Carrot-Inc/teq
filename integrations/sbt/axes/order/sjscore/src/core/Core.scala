package core

object Core {
  val n: Int = 1
  inline def twice(x: Int): Int = x * 2
  // A Scala 2 identifier, which Scala 3 made a keyword.
  val given: Int = 3
}
