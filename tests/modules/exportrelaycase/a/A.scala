package rca

object A:
  case class Box(n: Int)
  type Text = String
  class Cell(val value: Int)
  extension (s: String) def doubled: String = s + s

object B:
  export A.*

// `B`'s wildcard export relayed: the case class's companion, the type, the class and the
// extension are among `B`'s exports, so `C` forwards them, which a downstream reads from `C`'s
// pickle.
object C:
  export B.*
