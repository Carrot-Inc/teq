package p

object Old:
  @deprecated("old", "1") def v: Int = 1

object Defs:
  inline def get: Int = Old.v + 1
