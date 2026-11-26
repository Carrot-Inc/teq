import O.*

@main def run(): Unit =
  println((O.b, O.c, O.j, O.h, O.k))
  println((b, c, h))
  println(valueOf[O.c.type])
