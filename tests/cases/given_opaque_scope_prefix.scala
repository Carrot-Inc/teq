// An opaque type's implicit scope by the reference's prefix (dotty's `ImplicitRunInfo.collectParts`
// takes `t.dealias`, and an opaque alias dealiases through its defining scope's `this` alone): where
// the definition is visible, `T` and `this.T` are the underlying type, without the companion;
// `A.T` written through the object, a member's type read through a path that is not the enclosing
// `this` (`A.T.of(1)`, an import's `of(1)`, a top-level member read at the top level) and a type
// declared outside the definition stay opaque, the companion's givens and extensions theirs.
def cmp[X](a: X, b: X)(using o: Ordering[X]): Int = o.compare(a, b)
object A:
  opaque type T = Int
  object T:
    given Ordering[T] = Ordering.Int.reverse
    def of(i: Int): T = i
  def mkT(i: Int): T = i
  def inside = summon[Ordering[T]].compare(1, 2)
  def explicit = summon[Ordering[A.T]].compare(1, 2)
  def viaThis = summon[Ordering[this.T]].compare(1, 2)
  class In:
    def nested = summon[Ordering[T]].compare(1, 2)
  def viaOf = cmp(A.T.of(1), A.T.of(2))
  def viaLocalOf = cmp(T.of(1), T.of(2))
  def viaMkT = cmp(mkT(1), mkT(2))
  def viaAmkT = cmp(A.mkT(1), A.mkT(2))
  def viaImport = { import A.T.*; cmp(of(1), of(2)) }
object B:
  def outside = summon[Ordering[A.T]].compare(A.T.of(1), A.T.of(2))
opaque type Box = Int
object Box:
  given Ordering[Box] = Ordering.Int.reverse
  def of(i: Int): Box = i
  extension (b: Box) def show = "box"
def mk(i: Int): Box = i
object Other:
  def ext(i: Int): Box = Box.of(i)
def boxOf = cmp(Box.of(1), Box.of(2))
def boxTyped = { val b: Box = Box.of(1); val c: Box = Box.of(2); cmp(b, c) }
def boxInside = summon[Ordering[Box]].compare(1, 2)
def boxMk = cmp(mk(1), mk(2))
def boxOther = cmp(Other.ext(1), Other.ext(2))
@main def run(): Unit =
  println(List(A.inside, A.explicit, A.viaThis, (new A.In).nested, B.outside).mkString(" "))
  println(List(A.viaOf, A.viaLocalOf, A.viaMkT, A.viaAmkT, A.viaImport).mkString(" "))
  println(List(boxOf, boxTyped, boxInside, boxMk, boxOther).mkString(" "))
  println(Box.of(1).show)
