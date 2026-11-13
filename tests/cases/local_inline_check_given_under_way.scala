// A block's inline method called inside the right-hand side of a given of the block, before its definition: its
// body's search is the definition's, made where the block opens, so the given being defined around the call is a
// candidate like any other given of the block and is the nearest one. scalac resolves `summon[Mark]` to `local`.
trait Mark:
  def name: String

object Sought:
  given outer: Mark with
    def name: String = "outer"
  def run: String =
    var later: () => String = () => ""
    given local: Mark = {
      later = () => helper.name
      new Mark { def name: String = "local" }
    }
    inline def helper: Mark = summon[Mark]
    local.name + ", " + later() + ", " + helper.name

@main def run(): Unit = println(Sought.run)
