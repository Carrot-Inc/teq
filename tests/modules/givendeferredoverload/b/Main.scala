package gdob

import gdoa.*

class D extends C(using 7)

@main def run(): Unit =
  given Int = 4
  val c = new C
  println(c.x)
  println(c.x("s"))
  println((c: T).x)
  val d = new D
  println(s"${d.x} ${d.x("d")} ${(d: T).x}")
  println(M.fields[C])
  println(M.fields[D])
