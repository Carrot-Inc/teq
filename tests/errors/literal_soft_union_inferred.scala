// A join of branches is a soft union of their constants, widened where a value's type is inferred
// or a variable instantiated (`widenInferred`'s `Widen.Unions`: a transparent join keeps the union
// of the classes), on a plain inline method's inferred result too (`inferredResultType`).
object O:
  val m = if "a".isEmpty then 1 else "s"
  def d = if "a".isEmpty then 1 else 2L
  inline def i(inline b: Boolean) = if b then 1 else "s"
def run() =
  val l = if "a".isEmpty then 1 else "s"
  var w = if "a".isEmpty then true else 'c'
  val a: Int = O.m
  val b: Int = l
  val c: Int = w
  val d: Int = O.d
  val e: Int = O.i(true)
  val f = List(if "a".isEmpty then 1 else "s")
  val g: List[Int] = f
  val h = Option(if "a".isEmpty then 1 else "s")
  val k: Option[Int] = h
// expect: 11:16: error: type mismatch: found Int | String, required Int
// expect: 12:16: error: type mismatch: found Int | String, required Int
// expect: 13:16: error: type mismatch: found Boolean | Char, required Int
// expect: 14:16: error: type mismatch: found Long, required Int
// expect: 15:16: error: type mismatch: found Int | String, required Int
// expect: 17:22: error: type mismatch: found List[Int | String], required List[Int]
// expect: 19:24: error: type mismatch: found Option[Int | String], required Option[Int]
// expect: 7 errors found
