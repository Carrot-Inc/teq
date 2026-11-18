import scala.language.implicitConversions
class Ops[A](a: A):
  def m(b: A, c: Seq[A] = Seq(a)) = (b +: c).last

