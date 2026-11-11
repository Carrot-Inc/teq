// Two givens of one companion fit every target here, and each search ranks the one it tries first against the
// other: the same pair of candidates, compared for target after target.
trait Show[A]:
  def show(a: A): String

object Show:
  given any[A]: Show[A] with
    def show(a: A): String = "any"
  given list[A]: Show[List[A]] with
    def show(a: List[A]): String = "list"

def shown[A](a: A)(using s: Show[A]): String = s.show(a)

@main def run(): Unit =
  val seen = List(shown(List(1)), shown(List("a")), shown(List(1.0)), shown(List(List(1))), shown(Option(List(1))), shown(1))
  println(if seen == List("list", "list", "list", "list", "any", "any") then "passed" else seen.mkString(" "))
