package probe
object Use:
  val l = new Lib
  val a = l.f("a")
  val b = l.f(1)
  val c = l.f(1L)
  val d = l.f(1.0)
  val e = l.f(true)
  val f = l.m(1)
  val g = l.m("a")
  val h = Lib.f("a")
  val i = Lib.f(1)
  val j = CC("a")
  val k = CC(1)
  val o = l.opt.get
