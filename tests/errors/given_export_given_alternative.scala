// A given selector exports the given alternative of a name alone (dotty's
// `Namer.addWildcardForwarders`), so the method beside it is no member of the exporting object or
// class: scalac's E050 "given instance x in object E does not take parameters".
// expect: 15:11: error: value apply is not a member of Int
// expect: 16:11: error: value apply is not a member of Int
object O:
  given x: Int = 1
  def x(s: String): String = s
object E:
  export O.given
class V:
  val o: O.type = O
  export o.given
@main def run() =
  println(E.x("bad"))
  println(V().x("bad"))
