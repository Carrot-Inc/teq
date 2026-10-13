package ndb

@main def run(): Unit =
  val o = new nda.O(7)
  println(o.K())
  println(new o.K())
  println(o.K(1))
  println(o.K(y = "e"))
