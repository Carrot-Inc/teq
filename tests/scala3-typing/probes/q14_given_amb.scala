trait Show[A]:
  def show(a: A): String
object T:
  given a: Show[Int] with
    def show(a: Int) = a.toString
  given b: Show[Int] with
    def show(a: Int) = s"<$a>"
  def s = summon[Show[Int]].show(1)
@main def run(): Unit = println(T.s)
