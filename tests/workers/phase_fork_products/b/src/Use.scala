package probe
object Use:
  def plain: Int = Lib.inc(40)
  def wrapped: Int = Lib.wrap(41)
@main def run(): Unit =
  println(Use.plain)
  println(Use.wrapped)
