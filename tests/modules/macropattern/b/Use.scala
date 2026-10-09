package mpb

import mpa.Pat

@main def run(): Unit =
  val a = 1
  val b = 2
  println(Pat.kind(a + b))
  println(Pat.kind(List(a).head))
  println(Pat.kind(Some(a)))
  println(Pat.kind(Pat.plain(5)))
  println(Pat.kind("x"))
  println(Pat.typeKind[List[Int]])
  println(Pat.typeKind[Int])
  println(Pat.typeKind[Boolean])
