class C { def plus(x: Int): Int = x; def f(x: Int): Int = x; def g(x: Int => Int): Int = x(1); infix def h(x: Int): Int = x; def +++(x: Int): Int = x; def a_+(x: Int): Int = x }
object P {
  val c = new C
  val a = c f { 1 }
  val b = c f (1)
  val d = c `f` 1
  val e = c g ((x: Int) => x)
  val i = c g { (x: Int) => x }
  val j = c h 2
  val k = c +++ 3
  val l = c a_+ 4
  val m = c f 5
  val n = 1 max 2
  val o = List(1) map (_ + 1)
  val p = "a" concat "b"
  val q = c f (5 match { case x => x })
  val r = c.f(6)
  val s = c `plus` 7
}
