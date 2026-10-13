// A wildcard export forwards a name's alternatives by their givenness (dotty's
// `Namer.addWildcardForwarders`): a given selector the given alone, `*` the others, both in one
// clause both; a named selector all of them.
object O:
  given x: Int = 1
  def x(s: String): String = s
object E:
  export O.given
object F:
  export O.*
object G:
  export O.{given, *}
object H:
  export O.x
class V:
  val o: O.type = O
  export o.given
class W:
  val o: O.type = O
  export o.*
@main def run(): Unit =
  println(E.x)
  println(F.x("f"))
  println(s"${G.x}${G.x("g")}")
  println(s"${H.x}${H.x("h")}")
  import E.given
  println(summon[Int])
  println(s"${V().x} ${W().x("w")}")
