package ilcb

@main def run(): Unit =
  val a = ilca.API.make(7)
  val b = ilca.API.make(8)
  println(a())
  println(b())
