class C { infix def f(x: Unit): Unit = x }
object P {
  val c = new C
  var a = 0
  val t = c f (((a = 1)))
  val u = (((a = 2)))
  val v = c f ((a = 3))
}
