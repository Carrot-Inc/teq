// expect: 10:13: warning: value flag in object Casts is deprecated since 1: old
// expect: 10:13: warning: conversion from Boolean to Int will always fail at runtime.
// expect: 11:13: warning: conversion from Boolean to Int will always fail at runtime.
// expect: 3 warnings found, errors under --werror
// teq: --werror --deprecation
object Casts {
  @deprecated("old", "1") val flag: Boolean = true
  val other: Boolean = false
  def casts(): Unit = {
    println(flag.asInstanceOf[Int])
    println(other.asInstanceOf[Int])
  }
}
// scalac's conversion warning has no position (`tpd.primitiveConversion`), so the deprecation
// reported at `flag` before it, by an earlier phase, hides neither (`UniqueMessagePositions`):
// scalac reports the three warnings, the conversions after the deprecation.
