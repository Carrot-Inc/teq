// The join of the two pairs is a wildcard over `:=`'s `B` of each, which the body instantiates
// to `Builder`: the signature another worker reads names `Builder`.
object A:
  def mods(using c: Ctx) = List(
    Attr("class") := List("a" -> c.b),
    Attr("x") := "s"
  )
