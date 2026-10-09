package ilm

object Lazy:
  inline def f(): Int = { var n: Int = 0; val x: Int = { n += 1; n }; n }
