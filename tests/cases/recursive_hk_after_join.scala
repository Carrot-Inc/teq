// Equivalent higher-kinded arguments constructed before and after the body join
// retain the same representatives for sealed-case exhaustivity and union types.
sealed trait Branch[F[_]]
final case class Tip[F[_]](value: F[Int]) extends Branch[F]
final case class Fork[F[_]](left: Branch[F], right: Branch[F]) extends Branch[F]

object FirstBranch:
  def make: Branch[[A] =>> List[A]] = Tip(List(1))

object SecondBranch:
  def make: Branch[[B] =>> List[B]] = Fork(Tip(List(2)), Tip(List(3)))

object BranchSize:
  def size(value: Branch[[C] =>> List[C]]): Int = value match
    case Tip(values) => values.size
    case Fork(left, right) => size(left) + size(right)

// A recursive refinement is a RecType binding object, independently of the
// recursive class hierarchy above. Rebuild both kinds while resolving sealed cases.
type Row = AnyRef { type T; type U = T }
sealed trait Packet[A]
final case class Missing[A]() extends Packet[A]
final case class Present[A](value: A) extends Packet[A]
object PacketValue:
  val empty: Packet[AnyRef { type T; type U = T }] = Missing()
object PacketSize:
  def size(value: Packet[Row]): Int = value match
    case Missing() => 1
    case Present(_) => 2

@main def recursiveHkAfterJoin(): Unit =
  val both: Branch[[D] =>> List[D]] =
    if FirstBranch.make == SecondBranch.make then FirstBranch.make else SecondBranch.make
  println(BranchSize.size(both))
  println(PacketSize.size(PacketValue.empty))
