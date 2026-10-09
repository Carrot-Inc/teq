trait Show[A]:
  def show(a: A): String
trait Step:
  def next(i: Int): Int
inline def make[A](label: String, inline f: A => String): Show[A] =
  val pre = label + ":"
  new Show[A]:
    def show(a: A) = pre + f(a) + "!"
inline def stepper(k: Int): Step = i => i * k
inline def twice[A](inline f: A => String): (Show[A], Show[A]) = (make[A]("a", f), make[A]("b", f))
