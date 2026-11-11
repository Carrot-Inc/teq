// An alias that names another object's opaque type through that object (`P.T` from `Q`) is checked for cycles through
// the opaque type's hidden alias (CheckNonCyclicMap, Checking.scala:365-372), where one through its own object's `this`
// reads the self type's refinement (`integrateOpaqueMembers`): here neither cycles, and scalac accepts both.
object P:
  opaque type T = Int
  opaque type Pair = (T, T)
  def t(n: Int): T = n
  def pair(a: T, b: T): Pair = (a, b)
  def sum(p: Pair): Int = p._1 + p._2
object Q:
  type U = List[P.T]
  type V = Option[P.Pair]
  def us: U = List(P.t(1), P.t(2))
  def v: V = Some(P.pair(P.t(3), P.t(4)))
@main def run(): Unit =
  println(Q.us.size)
  println(Q.v.map(P.sum))
