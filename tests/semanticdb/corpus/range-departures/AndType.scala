class AndInv[A]
trait AndR[K] {
  def both: AndInv[K & CharSequence]
}
object AndType {
  val r: AndR[String] & AndR[Int] = ???
  val both = r.both
}
