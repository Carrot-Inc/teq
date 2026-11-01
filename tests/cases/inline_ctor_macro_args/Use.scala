class A(val s: Int):
  val l = Mac.lam(s)
  val w = Mac.wrap(s)
  val wc = Mac.wrap(s + 1)
  def viaMethod = Mac.lam(s)
class B(override val s: Int) extends A(s + 1)
@main def run(): Unit =
  val b = B(2)
  println(s"${b.l()} ${b.w()} ${b.viaMethod()} ${A(5).l()} ${A(5).w()} ${b.wc()} ${A(5).wc()}")
