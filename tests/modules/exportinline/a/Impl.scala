package eia

// An object's export of inline members: the forwarders are inline, the transparent one's
// transparent with its inline parameter, as scalac's `Exporter` makes them, so a downstream
// expands them and takes the transparent one's narrowed result.
object Impl:
  inline def twice(x: Int): Int = x + x
  inline val k = 3
  transparent inline def pick(inline b: Boolean): Any = inline if b then 1 else "s"

object Facade:
  export Impl.{twice, k, pick}
