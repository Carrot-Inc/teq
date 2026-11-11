// `+(-c.h.x)` in an inline body: unary_+ collapses into the negation's node, its typed record a selection over a
// retained copy of that node, whose own record is the negation's. The inline copy carries both, as TreeTypeMap
// transforms each tree and its qualifier (TreeTypeMap.scala:110, 117-123), so the expansion keeps the negation.
// scalac 3.8.4's output.
class C:
  def h: C = this
  val x: 42 = 42
object Inl:
  inline def plus(c: C): Int = +(-c.h.x)
  inline def widen(c: C): Long = c.h.x.toLong.toLong
  inline def neg(c: C): Int = -c.h.x
@main def main(): Unit =
  val c = new C()
  println(Inl.plus(c))
  println(Inl.widen(c))
  println(Inl.neg(c))
