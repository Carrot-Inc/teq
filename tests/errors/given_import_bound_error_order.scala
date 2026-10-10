// A clause's bound is the union of its `given` selectors' types from the left (`Namer.importBound`,
// `TypeComparer.lub`): an erroneous type after `Int` is dropped, as it conforms to it, so `P`'s clause
// brings the `Int`s alone (scalac's E172), and one ahead of `Int` drops it, so `Q`'s and the block's
// bring every given. Each missing type is reported once (E006).
object O { given int: Int = 1; given text: String = "t" }
object H { given h: Int = 2; given s: String = "s" }
object P:
  import O.{given Int, given Missing}
  def p = summon[String]
object Q:
  import H.{given Missing2, given Int}
  def q = summon[String]
@main def run(): Unit =
  locally { import H.{given NoSuchType, given String}; println(summon[Int]) }
  println(P.p + Q.q)

// expect: given_import_bound_error_order.scala:8:30: error: type Missing not found
// expect: given_import_bound_error_order.scala:9:25: error: no given instance of type String was found for parameter x
// expect: given_import_bound_error_order.scala:11:19: error: type Missing2 not found
// expect: given_import_bound_error_order.scala:14:29: error: type NoSuchType not found
// expect: 4 errors found
