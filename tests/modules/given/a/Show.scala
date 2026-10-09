package ga

trait Show[A]:
  def show(a: A): String

object Show:
  given Show[Int] with
    def show(a: Int): String = "int " + a
  given listShow[A](using s: Show[A]): Show[List[A]] with
    def show(as: List[A]): String = as.map(s.show).mkString("[", ", ", "]")
  given stringShow: Show[String] = (s: String) => "str " + s

def describe[A](a: A)(using s: Show[A]): String = s.show(a)
