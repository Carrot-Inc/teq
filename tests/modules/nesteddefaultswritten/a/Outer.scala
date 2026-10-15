package ndw

// A case class nested in a class beside its written companion, an object of each instance: the
// constructor's default getter is a member of that companion (`Desugar.defaultGetters` places it among
// the companion's members), numbered over the declared parameters, `Outer$Item$.$lessinit$greater$default$1()`,
// which a downstream of either compiler calls on `o.Item`, and the companion holds the synthesized
// `apply` and `unapply` as a companion the typer makes does.
class Outer {
  case class Item(hidden: Boolean = false)
  object Item {}
}
