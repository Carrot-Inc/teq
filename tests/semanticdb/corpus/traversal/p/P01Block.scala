package p01
object Blocks:
  def m: Int =
    val first = { val inner = 1; inner }
    def f(x: Int): Int = g(x) + first
    def g(y: Int): Int = y
    class Local(val a: Int) { def get = a }
    object LObj { val z = 3 }
    type T = Int
    val l: Local = new Local(first)
    l.get + f(2) + LObj.z
  def fwd: Int =
    def a: B = new B
    class B { def v = 1 }
    a.v
  def lzy: Int =
    lazy val lz = 1
    var vr = 2
    lz + vr
