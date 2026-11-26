// An export inside an extension group of a method whose name ends in ':': the forwarder's clauses are never swapped.
class Ops(val s: String):
  def %:(n: Int): String = s + n
object Lib:
  extension (s: String)
    def ops: Ops = new Ops(s)
    export ops.%:
@main def run(): Unit = println(Lib.%:("x")(7))
