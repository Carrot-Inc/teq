trait FromString[A]:
  def parse(s: String): A
trait Handler:
  def handle(x: Int): String

given FromString[Int] = _.toInt
val h: Handler = x => s"h$x"
def use(h: Handler) = h.handle(2)

@main def run(): Unit =
  println(summon[FromString[Int]].parse("12"))
  println(h.handle(1))
  println(use(x => "lam" + x))
