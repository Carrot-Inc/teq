// jars: scala-library givenfold-lib
// A library's givens summoned through scala-library's inline `summon` (a companion's and a nested object's among
// them): each is the given's own value at every worker count, its getter called as scalac's client calls it.
object Main:
  def main(args: Array[String]): Unit =
    import givenfold.Givens.given
    import givenfold.C.given
    import givenfold.Outer.Inner.given
    println(s"${summon[Int]} ${summon[Long]} ${summon[Boolean]} ${summon[String]} ${summon[Char]} ${summon[Double]}")
