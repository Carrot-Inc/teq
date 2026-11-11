package s5

object O:
  var base: Int = 1
  inline def m(inline x: Int = base + 3)(y: Int = base + 4): Int = x * 10 + y

trait T:
  export O.{m as mm}
  def tk: Int = 1
  inline def g[A](inline a: A, inline x: Int = tk + 1)(using s: String = "u")(z: Int = 9): String = s"$a$x$s$z"

trait T2 extends T:
  override def tk: Int = 10
  def f: Int = super.mm()()
  def h: String = super.g[Int](1)()

class C extends T2
