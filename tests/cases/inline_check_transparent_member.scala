// A transparent inline member is expanded where its call is typed, so its body is checked for the first call that
// meets it, and the check runs as the member stands in its class: a case round that call refines the class's type
// parameter for the caller alone, and the given whose initialiser holds the call is a candidate of the member's own
// search like any other given of its class. scalac types the member's body in the context it was made in: the `Any`
// alternative for a value of the class's parameter, and the nearest given, which is the one being defined.
sealed trait Tag[A]
case object IntTag extends Tag[Int]
case object StrTag extends Tag[String]

class Keeper[A](held: A):
  def which(x: Int): String = "int " + (x + 1)
  def which(x: Any): String = "any"
  def run(tag: Tag[A]): String =
    val first = tag match
      case IntTag => shown
      case _ => "other"
    first + ", " + shown
  transparent inline def shown: String = which(held)

trait Mark:
  def name: String

trait Farther:
  given outer: Mark with
    def name: String = "outer"

object Chosen extends Farther:
  def wrap(m: => Mark): Mark = new Mark:
    def name: String = if m eq this then "local" else m.name
  given local: Mark = picked
  transparent inline def picked: Mark = wrap(summon[Mark])

@main def run(): Unit =
  println(new Keeper(2).run(IntTag))
  println(new Keeper("b").run(StrTag))
  println(Chosen.local.name)
