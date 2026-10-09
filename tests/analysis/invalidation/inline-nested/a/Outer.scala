package iln

object Outer:
  inline def outer: Int = Inner.inner + 1
