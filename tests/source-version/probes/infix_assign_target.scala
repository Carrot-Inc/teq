class C {
  infix def f(x: Unit): Unit = x
  var x: Int = 0
}
@main def run(): Unit = {
  val c = new C
  c f (c.x = 1)
  println(c.x)
}
