class C {
  infix def f(x: (a: Int, b: Int)): Int = x.a
  infix def k(t: (Int, Int)): Int = t._2
}
@main def run(): Unit = {
  val c = new C
  var a = 0
  println(c f ((a = 1, b = 2)))
  println(c k ((3, 4)))
  val t = ((a = 5))
  println(t)
}
