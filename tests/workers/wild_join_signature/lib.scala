class Builder
trait AttrValue[B, T]
case class Attr(name: String):
  def :=[B, T](v: T)(implicit ev: AttrValue[B, T]): AttrPair[B, T] = AttrPair(this, v, ev)
case class AttrPair[B, T](a: Attr, v: T, ev: AttrValue[B, T])
given AttrValue[Builder, String] = new AttrValue[Builder, String] {}
given AttrValue[Builder, List[(String, Boolean)]] = new AttrValue[Builder, List[(String, Boolean)]] {}
case class Ctx(b: Boolean)
