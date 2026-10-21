// teq: --werror
// `adaptToSubType`'s `convertTo` against the expected type itself: `classBound` strips a `Null` alternative alone
// (`stripNull`), so `Byte | Null` and `Char | Null` take the narrowed constant, and the lossy widening's warning reads
// the expected type's `classSymbol` (`Float | Null`'s is its join's, no `Float`): clean under `-Werror` as scalac 3.8.4
// compiles it; a union with another member takes the view (`Long | String`, `Double | String`). The review of
// `4fd7f3293`, item 3.
def id[T](x: T): T = x
val b: Byte | Null = id[65](65)
val c: Char | Null = id[65](65)
val f: Float | Null = id[16777217](16777217)
val g: Float | Null = 16777217
val l: Long | String = 65
val d: Double | String = id[65](65)

// The floats printed by their integral values, which every platform writes alike.
@main def run(): Unit =
  val (fi, gi) = (f.asInstanceOf[Float].toInt, g.asInstanceOf[Float].toInt)
  println(s"$b $c $fi $gi $l ${d == 65.0}")
