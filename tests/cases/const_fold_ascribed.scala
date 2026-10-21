// `ConstFold.ConstantTree` of a `Typed(expr, _)`: the inner tree read with its own typed type, a generic call's
// instantiated result included, and `Literal(null)` the null constant `foldBinop` compares (the review of
// `4f8b8bb39`'s GenericAscribed and NullAscribed).
object O {
  def id[T](x: T): T = x
  val generic: "xy" = (id["x"]("x"): String) + "y"
  val nullEq: true = (null: Null) == null
  val nullNe: false = (null: Null) != null
  val strEq: false = ("a": String) == null
}

@main def run(): Unit =
  println(s"${O.generic} ${O.nullEq} ${O.nullNe} ${O.strEq}")
