object Lib:
  inline def f(x: Int): Int = scala.compiletime.error("boom")
  class Box[A <: String]
