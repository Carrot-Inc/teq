// An inline method of a trait with a default getter,
// `tm` with a plain parameter's and `tn` with
// its second clause's: scalac 3.8.4's interface T
// declares the getters and their static forwarders
// and no `tm` or `tn` (the .expected). Found by
// reviewer #13 of the workers exports helper (push 8).
//> using platform jvm
trait T:
  inline def tm(x: Int = 5): Int = x * 2
  inline def tn(inline a: Int = 1)(b: Int = 2): Int = a + b
  def plain: Int = tm() + tn()()

class C extends T

@main def run(): Unit =
  println(C().plain)
  println(classOf[T].getDeclaredMethods.map(_.getName).sorted.mkString(","))
