class Ops(val s: String):
  def g(a: Int)(b: Int = 2, c: Int = 3): Int = a * 100 + b * 10 + c + s.length * 1000
object W:
  extension (s: String)
    def ops: Ops = Ops(s)
    export ops.g
object Q:
  export W.g
@main def run(): Unit =
  import Q.*
  println("xy".g(2)(7, 8))
