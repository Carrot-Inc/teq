// jars: scala-library
// std: scala-library
// A given object of a package or of objects in scalac's layout:
// its module class, made once on the first read, which does not initialise its owner (`Show initialised` is
// never printed, as under scalac); a top-level one and one of a nested object.
trait Show[A]:
  def show(a: A): String
object Show:
  println("Show initialised")
  given Show[Int] with
    println("int given made")
    def show(a: Int): String = s"int $a"
  given named: Show[Long] with
    val prefix = "L"
    def show(a: Long): String = prefix + a
given Show[String] with
  def show(a: String): String = s"str $a"
object Holder:
  object Inner:
    given Show[Boolean] with
      def show(a: Boolean): String = if a then "yes" else "no"
def render[A](a: A)(using s: Show[A]): String = s.show(a)
object Main:
  def main(args: Array[String]): Unit =
    println("start")
    println(render(1))
    println(render(2))
    println(summon[Show[Int]] eq summon[Show[Int]])
    println(Show.named.show(3L))
    println(render("x"))
    import Holder.Inner.given
    println(render(true))
    val shows: List[Show[Int]] = List(summon[Show[Int]])
    println(shows.map(_.show(4)))
