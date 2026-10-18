class C { infix def g(t: (a: Int)): Int = t.a }
object P {
  val c = new C
  var a = 0
  val t = c g (((a = 1)))
  val u = (((a = 2)))
}
