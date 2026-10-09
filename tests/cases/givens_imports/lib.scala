package lib

// Type classes with extension methods, and givens spread over objects the way libraries do it.

trait Enumerated[E]:
  def names: List[String]
  extension (e: E)
    def entryName: String
    def entryNameUpper: String = e.entryName.toUpperCase

trait HasLabel[A]:
  extension (a: A) def label: String

trait Reusability[A]:
  def test(a: A, b: A): Boolean
final class ByEq[A] extends Reusability[A]:
  def test(a: A, b: A): Boolean = a == b

trait LowPriorityReusability:
  given anyReusability[A]: Reusability[A] = ByEq()

object Reusability extends LowPriorityReusability:
  def by_==[A]: Reusability[A] = ByEq()
  object MapImplicits:
    given reusabilityMap[K, V]: Reusability[Map[K, V]] = ByEq()
    def notAGiven: String = "plain member"
  trait Instances:
    given inheritedInt: Reusability[Int] = ByEq()
  object All extends Instances:
    given ownString: Reusability[String] = ByEq()

trait Monoid[A]:
  def empty: A
  def combine(a: A, b: A): A

given intMonoid: Monoid[Int] with
  def empty: Int = 0
  def combine(a: Int, b: Int): Int = a + b

def packageHelper: String = "package helper"
