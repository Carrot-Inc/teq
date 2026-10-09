object Lib:
  inline def f(x: Int): Int =
    val b: Box[Int] = null
    x + 1
  class Box[A <: String]
