class NestedInline {
  inline def inner(inline x: Int): Int = x + 1
  inline def outer(inline y: Int): Int = inner(y)
  val a = outer(b)
  val b = 2
}
