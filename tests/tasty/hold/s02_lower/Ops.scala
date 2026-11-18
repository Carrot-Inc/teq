import scala.language.implicitConversions
class Ops[A](a: A):
  def m(b: A) = Option(b).getOrElse(a)

