package lib
object Impl:
  inline def t(inline x: Int = 3): Int = x * 2
export Impl.t
