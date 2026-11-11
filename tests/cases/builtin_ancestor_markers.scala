trait Tag
class Plain(val value: Int)
class Tagged extends Tag
def inspect(x: Any): Unit =
  println(x.isInstanceOf[AnyRef])
  println(x.isInstanceOf[Matchable])
  println(x.isInstanceOf[Tag])
@main def run(): Unit =
  val p = new Plain(7)
  println(p.value)
  inspect(p)
  inspect(new Tagged)
