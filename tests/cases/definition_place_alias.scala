// A type member of a class made in a block is first named inside a case that refines a type parameter its
// right-hand side goes through: the alias is completed in its own place, where the case's bound is not, so
// `Local#Later` is `a.Out` for every reader and not the `Int` the case would make of it. scalac prints first any.
sealed trait Tag[A]
trait Base:
  type Out
class Boxed extends Base:
  type Out = Int
case object BoxedTag extends Tag[Boxed]

object Aliased:
  def name(x: Int): String = "int"
  def name(x: Any): String = "any"
  def run[A <: Base](tag: Tag[A], a: A): String =
    def early: String = tag match
      case BoxedTag =>
        val x: Local#Later = null.asInstanceOf[Local#Later]
        "first"
    class Local:
      type Later = a.Out
    early + " " + name(null.asInstanceOf[Local#Later])

@main def run(): Unit =
  println(Aliased.run(BoxedTag, new Boxed))
