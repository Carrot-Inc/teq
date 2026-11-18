sealed trait Box[A]
case class IntBox(x: Int) extends Box[Int]

object Holder {
  def get[A](b: Box[A]): A = b match { case IntBox(x) => x }
  val v: Int = get(IntBox(1))
}
