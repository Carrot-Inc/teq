package ndm

// The control of `nesteddefaultswritten`: the same case class nested in a class without a written
// companion, whose companion the typer makes (`Desugar.classDef`'s `companionDefs`), its default getter
// `Outer$Item$.$lessinit$greater$default$1()` the same member.
class Outer {
  case class Item(hidden: Boolean = false)
}
