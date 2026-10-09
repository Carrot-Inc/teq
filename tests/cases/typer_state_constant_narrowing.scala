//> using platform js
// A plain inline call of a constant type narrows to the `Byte`, `Short`, `Char` or `Float` expected,
// as its literal does: the narrowing reads its expansion.
object M:
  inline def one: 1 = 1
  inline def a: 'a' = 'a'

@main def main(): Unit =
  val b: Byte = M.one
  val s: Short = M.one
  val c: Char = M.one
  val f: Float = M.one
  val ab: Byte = M.a
  println(s"$b $s ${c.toInt} ${f * 2} $ab")
