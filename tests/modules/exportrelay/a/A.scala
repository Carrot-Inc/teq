package era

trait Eqv[A]:
  def eqv(x: A, y: A): Boolean

trait Named[E]:
  def name(e: E): String

object Codecs:
  given namedEqv: [T: Named] => Eqv[T] = (x: T, y: T) => summon[Named[T]].name(x) == summon[Named[T]].name(y)
  def describe[T: Named](t: T): String = "named " + summon[Named[T]].name(t)

object Utils:
  export era.Codecs.{namedEqv, describe}

trait Kinds:
  type Label = String

object Vocab extends Kinds
