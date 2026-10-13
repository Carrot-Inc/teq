// A type variable's lower bounds joined as dotty's lub joins them: of two types each below the
// other the first stays (`mergedLub`, `ConstraintHandling.addOneBound`'s `lo | bound`), so an
// opaque type's external reference `A.T` first keeps its prefix against `A.this.T` after it, and
// the implicit scope is the external one's.
def cmp[X](a: X, b: X)(using o: Ordering[X]): Int = o.compare(a, b)
object A:
  opaque type T = Int
  object T:
    given Ordering[T] = Ordering.Int.reverse
    def of(i: Int): T = i
  def id[X](x: X): X = x
  def run(): Unit =
    println(cmp(A.T.of(1), A.T.of(2)))
    println(cmp(id(A.T.of(1)), id(A.T.of(2))))
    println(cmp(A.T.of(1), T.of(2)))
    println(cmp(T.of(1), A.T.of(2)))
    println(cmp(T.of(1), T.of(2)))
@main def run() = A.run()
