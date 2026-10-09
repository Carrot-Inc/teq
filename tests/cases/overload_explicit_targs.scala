// Two alternatives that both fit the arguments, one taking the subclass: the one whose
// parameter types the other accepts is the more specific, with explicit type arguments as
// without them (dotc's isAsSpecific); a secondary constructor against the primary too.
sealed abstract class Chain[+A]
object Chain:
  sealed abstract class NonEmpty[A] extends Chain[A]
  case object Empty extends Chain[Nothing]
  final case class Singleton[A](a: A) extends NonEmpty[A]
  final case class Append[A](leftNE: NonEmpty[A], rightNE: NonEmpty[A]) extends NonEmpty[A]
  object Append:
    def apply[A](left: Chain[A], right: Chain[A]): Append[A] = (left, right) match
      case (l: NonEmpty[A], r: NonEmpty[A]) => Append(l, r)
      case _ => throw new IllegalArgumentException("empty")
  def concat[A](c: Chain[A], c2: Chain[A]): Chain[A] = (c, c2) match
    case (x: NonEmpty[A], y: NonEmpty[A]) => Append[A](x, y)
    case (Empty, y) => y
    case (x, _) => x
  class ChainIterator[A](self: NonEmpty[A]):
    def this(chain: Chain[A]) = this(chain match
      case ne: NonEmpty[A] => ne
      case _ => Singleton(null.asInstanceOf[A]))
    def kind: String = self match
      case Append(_, _) => "append"
      case Singleton(_) => "single"

@main def run(): Unit =
  val c = Chain.concat(Chain.Singleton(1), Chain.Singleton(2))
  println(c)
  println(Chain.concat(Chain.Empty, Chain.Singleton(3)))
  val app = Chain.Append(Chain.Singleton(1), Chain.Singleton(2))
  println(new Chain.ChainIterator[Int](app).kind)
  println(new Chain.ChainIterator(Chain.Singleton(1): Chain[Int]).kind)
