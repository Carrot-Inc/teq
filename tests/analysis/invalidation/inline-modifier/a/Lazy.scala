package ilm

object Lazy:
  inline def f(): Int = { var n: Int = 0; lazy val x: Int = { n += 1; n }; n }
