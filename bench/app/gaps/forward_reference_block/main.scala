// scalac rejects a forward reference to a `def` of the same block when a `val` in between reads
// it ("forward reference extends over the definition of density"); teq accepts and runs it.
object Main:
  def main(args: Array[String]): Unit =
    val density = fields.length match
      case 0 => "empty"
      case _ => "full"
    def fields = List("a", "b")
    println(density)
