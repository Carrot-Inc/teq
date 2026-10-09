// expect: 11:3: warning: conversion from Boolean to Int will always fail at runtime.
// expect: 12:3: warning: conversion from Int to Boolean will always fail at runtime.
// expect: 13:3: warning: conversion from Unit to Int will always fail at runtime.
// expect: 3 warnings found, errors under --werror
// absent: conversion from Char
// teq: --werror
// scalac warns of a primitive cast to a primitive that no conversion makes
// (`tpd.primitiveConversion`), at its erasure and with no position; teq where the cast stands.
// A cast that converts is no warning.
def casts(b: Boolean, i: Int, u: Unit, c: Char): Unit =
  b.asInstanceOf[Int]
  i.asInstanceOf[Boolean]
  u.asInstanceOf[Int]
  c.asInstanceOf[Int]
  i.asInstanceOf[Char]
