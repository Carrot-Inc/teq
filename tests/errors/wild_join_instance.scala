// The join of two applications whose invariant argument is an open variable each (`:=`'s `B`,
// which its interpolation leaves open as dotty's leaves a nonvariant one) is a wildcard over the
// two (`wild_join`); the body instantiates them, and the inferred type names their instance as
// scalac's does (`zonk` through the wildcard's bounds), not the variables.
class Builder
trait AttrValue[B, T]
case class Attr(name: String):
  def :=[B, T](v: T)(implicit ev: AttrValue[B, T]): AttrPair[B, T] = AttrPair(this, v, ev)
case class AttrPair[B, T](a: Attr, v: T, ev: AttrValue[B, T])
given AttrValue[Builder, String] = new AttrValue[Builder, String] {}
given AttrValue[Builder, List[(String, Boolean)]] = new AttrValue[Builder, List[(String, Boolean)]] {}
def mods(b: Boolean) = List(Attr("class") := List("a" -> b), Attr("x") := "s")
val x: Int = mods(true)
// expect: 13:14: error: type mismatch: found List[AttrPair[Builder, ? >: List[(String, Boolean)] & String <: (List[(String, Boolean)] | String)]], required Int
