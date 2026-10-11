// Calls that leave arguments to their defaults: trailing ones, an earlier clause's, a using clause's,
// a constructor's and an inherited one's, with literal and effectful arguments written in and out
// of parameter order. The plan binds a temporary only where scalac's does; an omitted trailing
// default is the callee's to compute.
final case class Options(
    name: String,
    retries: Int = 3,
    verbose: Boolean = false,
    prefix: String = "[app]",
    width: Int = 80,
    tags: List[String] = Nil
)

trait Shape:
  def describe(label: String = "shape", precision: Int = 2): String

final class Circle(r: Double) extends Shape:
  def describe(label: String, precision: Int): String = s"$label r=${(r * math.pow(10, precision)).round}e-$precision"

object Log:
  var lines = 0
  def entry(msg: String, level: String = "info", indent: Int = 0)(using ctx: String = "main"): String =
    lines += 1
    (" " * indent) + s"$ctx/$level: $msg"
  def window(start: Int = 0)(size: Int, step: Int = 1): List[Int] = (start until (start + size) by step).toList

def note(s: String): String =
  println(s"note $s")
  s

@main def run(): Unit =
  println(Options("a"))
  println(Options("b", verbose = true))
  println(Options(width = 120, name = note("c")))
  println(Options("d", tags = List("x", "y"), retries = 5))
  println(Options("e").copy(prefix = "[e]"))
  println(Options(prefix = note("[f]"), name = note("f"), width = 40))
  val shapes: List[Shape] = List(Circle(1.0), Circle(2.5))
  shapes.foreach(s => println(s.describe()))
  shapes.foreach(s => println(s.describe(precision = 3)))
  println(Log.entry("start"))
  println(Log.entry("warned", level = "warn"))
  println(Log.entry(indent = 2, msg = note("nested")))
  println(Log.entry("ctx")(using "other"))
  println(Log.window()(5))
  println(Log.window(10)(6, 2))
  println(Log.window()(step = 3, size = 9))
  println(Log.lines)
