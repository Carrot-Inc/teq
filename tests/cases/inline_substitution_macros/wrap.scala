import M.*
object W:
  inline def twiceShape(inline y: Int): String = shape(y + y)
  inline def pairType[A](inline a: A): String = typeOf((a, a))
  inline def whereWrapped(inline y: Int): String = where(y)
  inline def symWrapped(inline y: Int): String = symbolOf(y)
