package rob

class O3 extends roa.O2

@main def run() =
  val o = new O3
  println(o.toString == "wrong")
  println(o.toString(1))
  val o1: roa.O1 = o
  println(o1.toString == "wrong")
