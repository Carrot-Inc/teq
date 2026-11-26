// An export inside an extension group of a method whose name ends in ':', selected on the receiver.
object Lib:
  extension (x: String)
    def plus(y: Int): String = x + y
object Pub:
  export Lib.{plus as %:}
@main def run(): Unit =
  import Pub.*
  println("x".%:(7))
