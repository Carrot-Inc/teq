class C {
  infix def h(u: Unit): Int = 1
  infix def g(t: (Unit, Unit)): Int = 2
}
@main def run(): Unit = {
  val c = new C
  var a = 0
  var b = 0
  val t = ((a = 1))
  println(a)
  println(c h ((a = 2)))
  println(a)
  println(c g ((a = 3, b = 4)))
  println((a, b))
}
