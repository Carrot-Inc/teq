// An applied opaque type written through its object's path inside the object, `A.T[String]`, is
// dotty's external reference `TypeRef(TermRef(A), T)` applied: opaque for the implicit scope, its
// companion's given found, where `T[String]` (`A.this.T`) is its underlying type; both conform to
// that type there (`TypeComparer.liftToThis`).
object A:
  opaque type T[X] = Int
  object T:
    given [X]: Ordering[T[X]] = Ordering.Int.reverse
  val v: A.T[Long] = 3
  val w: Int = v
  def run(): Unit =
    println(summon[Ordering[A.T[String]]].compare(1, 2))
    println(summon[Ordering[T[String]]].compare(1, 2))
    println(summon[Ordering[Option[A.T[Int]]]].compare(Some(1), Some(2)))
    println(w)
@main def run() = A.run()
