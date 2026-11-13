package fsa

// An object's export of a val and of an object: the forwarders are stable methods, paths as their members
// are, and another module names them in singleton types.
object Stock:
  val plain: String = "p"
  object Inner:
    val n = 9

object Counter:
  export Stock.{plain, Inner}
