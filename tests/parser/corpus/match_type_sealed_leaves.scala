// A sealed class or trait without children has no instances but its own, so a match type
// reduces past a case it does not derive from, whatever its type arguments.
sealed trait Snap
object Snap:
  sealed trait None extends Snap
  sealed trait Some[A] extends Snap
  type Value[U <: Snap] = U match
    case None    => Unit
    case Some[a] => a

sealed class Leaf
trait Mark
type Kind[X] = X match
  case Mark => "mark"
  case Leaf => "leaf"

final class Box[X](val x: X)
def setLC[U <: Snap](b: Box[Snap.Value[U]]): Box[Snap.Value[U]] = b
def nested[US <: Snap](b: Box[Snap.Value[US]]): Box[Snap.Value[US]] = setLC[Snap.Some[Snap.Value[US]]](b)
def lift[X](x: X): Snap.Value[Snap.Some[X]] = x

@main def run(): Unit =
  println(nested[Snap.Some[String]](Box("a")).x)
  println(lift(4) + 1)
  val k: Kind[Leaf] = "leaf"
  println(k)
