package fp

object Fp:
  inline def s: String = "one"
  inline def outer: Int = inner + 1
  inline def inner: Int = 1
