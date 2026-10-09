trait Show[A]:
  def show(a: A): String
trait Handler:
  def handle(x: Int): String
given Show[Int] = _.toString
val h: Handler = x => s"h$x"
def use(h: Handler) = h.handle(2)
@main def run(): Unit =
  println(summon[Show[Int]].show(1))
  println(h.handle(1))
  println(use(x => "lam" + x))
