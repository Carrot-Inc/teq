package mca

object Macros:
  inline def m: Int = ${ Impl.code }
