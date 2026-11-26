// An exported method whose name ends in ':' is no extension: its clauses stay as they are.
object Lib:
  extension (x: String)
    def plus(y: Int): String = x + y
object Pub:
  export Lib.{plus as %:}
@main def run(): Unit = println(Pub.%:("x")(7))
