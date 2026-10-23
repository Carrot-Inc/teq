class Invariant[A]
object Meet {
  def distributed: Invariant[(Int | String) & (Int | Boolean)] = ???
  val distribution = distributed
}
