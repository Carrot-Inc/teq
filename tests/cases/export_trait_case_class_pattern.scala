// A case class and its companion exported by a trait that a prelude object extends: the
// companion's forwarder is a stable path to the object (dotty's `StableRealizable` forwarder),
// so a constructor pattern through it is the case class's, as is one through an export chain.
object data:
  final case class Nel[+A](head: A, tail: List[A])

trait SharedPrelude:
  export data.Nel

object AppPrelude extends SharedPrelude

object Chained:
  export AppPrelude.{*, given}

enum AllOrOne[+T]:
  case All
  case One(value: T)

enum Pickup:
  case Drivethru, Curbside

object ByImport:
  import AppPrelude.*
  def pick(n: Nel[AllOrOne[Pickup]]): Option[Pickup] = n match
    case Nel(AllOrOne.One(p), Nil) => Some(p)
    case _                         => None

object ByChain:
  import Chained.*
  def pick(n: Nel[AllOrOne[Pickup]]): Option[Pickup] = n match
    case Nel(AllOrOne.One(p), Nil) => Some(p)
    case _                         => None

def bySelect(n: data.Nel[Int]): Int = n match
  case AppPrelude.Nel(h, _) => h

@main def run(): Unit =
  val n = data.Nel(AllOrOne.One(Pickup.Curbside), Nil)
  println(ByImport.pick(n))
  println(ByChain.pick(data.Nel(AllOrOne.All, Nil)))
  println(bySelect(data.Nel(5, Nil)))
