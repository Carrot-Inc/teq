// A class nested in a class takes its enclosing instance first; its constructor's defaults are
// those of the parameters after it, inside the enclosing class and through a prefix.
class Outer(val tag: String):
  case class Opt(a: Int = 1, b: String = "x"):
    def show = s"$tag:$a:$b"
  class Plain(val n: Int = 7)(val m: Int = n * 2):
    def show = s"$tag:$n:$m"
  def inside = List(Opt().show, Opt(2).show, Opt(b = "y").show, new Plain()().show)

@main def run(): Unit =
  val o = Outer("o")
  println(o.inside)
  println(List(o.Opt().show, o.Opt(5, "z").show))
