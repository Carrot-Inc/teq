class C(n: Int) {
  val f = g
  def g = m + n
  val m = n
}

abstract class A(val x: Int) { val y = show; def show = x }
class B extends A(1) { override val x = 5 }
