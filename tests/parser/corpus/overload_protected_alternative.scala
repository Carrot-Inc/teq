// An overloaded method whose alternative is protected: a call from outside the class sees the
// public alternative alone (zio's `someOrElse`).
class Box[A](val a: Option[A]):
  protected final def orElse[B](default: => B)(using ev: A <:< B): B = a.fold(default)(ev)
  final def orElse[B, C](default: => C)(using ev0: A <:< B, ev1: C <:< B): B = a.fold(ev1(default))(ev0)
object Main:
  def main(args: Array[String]): Unit =
    println(Box(Some("x")).orElse("y"))
    println(Box[String](None).orElse("z"))
