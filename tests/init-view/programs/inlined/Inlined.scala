class I {
  inline def twice(inline x: Int): Int = x + x
  inline def once(y: Int): Int = y
  val a = twice(b)
  val c = once(b)
  val b = 3
}
