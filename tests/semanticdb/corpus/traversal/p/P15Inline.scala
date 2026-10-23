package p15
object Inl:
  inline def twice(inline x: Int): Int = x + x
  transparent inline def tr: Any = 1
  val a = twice(2)
  val b = tr
  def c = { val q = 1; twice(q) }
