object Use:
  val a: Int = Lib.f(1, 2)
  val b: Int = Lib.f()
  val c: Int = Lib.g(1, 2, 3)
  val d: String = Lib.h("s")
  val e: Int = Lib.h(1, 2)
  val k = new Lib.K(1, 2, 3)
  val kk: Int = k.k + k.base + k.missing
  val p = Lib.P(1)
  val q = p.copy(x = 3)
  val bad: String = 1
