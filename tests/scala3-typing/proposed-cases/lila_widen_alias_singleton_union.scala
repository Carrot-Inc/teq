// lila: TutorPerfUi: active.has(angle) with active: Option[Angle],
// type Angle = "skills" | "opening" | ..., and `has[A](self: Option[A])(b: A)(using Eq[A])`.
// A has the lower bound Angle and is instantiated before the search for Eq[A]; scalac widens
// the alias of a union of string literal types to String when instantiating it from below
// (ConstraintHandling.widenInferred), so the search is for Eq[String] and finds it.
trait Eq[A] { def eqv(x: A, y: A): Boolean }
object Eq:
  given Eq[String] = _ == _

extension [A](self: Option[A]) def has(b: A)(using eq: Eq[A]): Boolean = self.exists(eq.eqv(_, b))

type Angle = "skills" | "opening" | "time"

@main def run() =
  val active: Option[Angle] = Some("time")
  val angles: List[Angle] = List("skills", "time")
  println(angles.map(a => a -> active.has(a)))
