// expect: missing argument list for method vararg in object T
object T:
  def vararg(xs: Int*): Int = xs.sum

@main def run(): Unit =
  val g9 = T.vararg
  val ok: Seq[Int] => Int = T.vararg
  println(ok(Seq(1, 2)))
