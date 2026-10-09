// A bare reference to a polymorphic method that a member is selected from leaves the type
// arguments its result does not fix open: the expected type of the whole expression fixes them.
final case class Lens[A, B](get: A => B)(val set: B => A => A):
  def mod(f: B => B): A => A = a => set(f(get(a)))(a)

final case class Holder[F[_]](slot: Option[Int])

object Holders:
  def slotL[F[_]] = Lens((h: Holder[F]) => h.slot)(n => _.copy(slot = n))
  def pair[A]: (List[A], Int) = (Nil, 0)

@main def run(): Unit =
  val bump: Holder[List] => Holder[List] = Holders.slotL.mod(_.map(_ + 1))
  println(bump(Holder[List](Some(1))))
  val none: Holder[Option] => Holder[Option] = Holders.slotL.mod(_ => None)
  println(none(Holder[Option](Some(5))))
  val size: Int = Holders.pair._2
  println(size)
