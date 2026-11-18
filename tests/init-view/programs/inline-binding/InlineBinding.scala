class InlineBinding {
  inline def inner(x: Int): Int = x
  inline def outer(inline y: Int): Int = inner(y + 1)
  val a = outer(b)
  val b = 2
}
