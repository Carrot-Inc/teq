package gopa

// An opaque type's references by prefix inside its defining object, pickled and read over the
// products: an inferred result reached through the object's path names it externally, one read
// through the object's `this` transparently; outside, both are the opaque type.
def cmp[X](a: X, b: X)(using o: Ordering[X]): Int = o.compare(a, b)
object A:
  opaque type T = Int
  object T:
    given Ordering[T] = Ordering.Int.reverse
    def of(i: Int): T = i
  def external(i: Int) = A.T.of(i)
  def internal(i: Int) = T.of(i)
  def insideExternal = cmp(external(1), external(2))
  def insideInternal = cmp(internal(1), internal(2))
// An applied one written through its object's path keeps the path in its declared result.
object B:
  opaque type F[X] = Int
  object F:
    given [X]: Ordering[F[X]] = Ordering.Int.reverse
    def of[X](i: Int): F[X] = i
  def applied(i: Int): B.F[String] = F.of(i)
  def insideApplied = summon[Ordering[B.F[String]]].compare(1, 2)
  def insideBare = summon[Ordering[F[String]]].compare(1, 2)
