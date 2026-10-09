// A named selector of an import through a value of a builtin type is taken as it was before the
// selectors were checked (`Checking.checkImportSelectors` checks a class's own type here): a number's
// conversions are the typer's primitives, no member a class declares (`n.toLong`, `n.toDouble`).
class UseInt(val n: Int):
  import n.toLong
  def result: Int = n

class UseDouble(val d: Double):
  import d.{toInt, toFloat}
  def result: Double = d * 2

@main def run(): Unit =
  println(new UseInt(7).result)
  println(new UseDouble(1.25).result == 2.5)
