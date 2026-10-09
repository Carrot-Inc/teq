package fix.guard

import scala.util.NotGiven

opaque type Step = () => String

// A class and an object of one name nested in an object, the object's given `apply` taking a
// proof whose own given takes inline `NotGiven` evidence: a call of `Step(...)` passes both.
object Step:
  final class Guard[A] private[Step]()
  object Guard:
    final class Proof[A] private[Step]()
    object Proof:
      inline given legal[A](using inline ev1: NotGiven[A <:< Step], inline ev2: NotGiven[() => A]): Proof[A] = (null: Proof[A])
    inline given apply[A](using inline ev: Proof[A]): Guard[A] = (null: Guard[A])
  inline def apply[A](inline f: A)(using inline ev: Guard[A]): Step = () => "ran " + f
  extension (s: Step) def run(): String = s()

class Steps:
  def twice(n: Int): Step = Step(n * 2)
