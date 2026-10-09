// A hiding selector is used when it hides a name a use would have reached through the wildcard.
object Lib { val a = 1; val b = 2 }
object Hiding {
  import Lib.{a as _, *}
  val x = b
}
object Hides {
  val a = "outer"
  object In {
    import Lib.{a as _, *}
    val y = a + b
  }
}
