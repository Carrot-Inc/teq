// An export inside an extension group of a plain method, called written out.
object Lib:
  extension (x: String)
    def %:(y: Int): String = x + y
object Pub:
  export Lib.{%: as plus}
@main def run(): Unit = println(Pub.plus(7)("x"))
