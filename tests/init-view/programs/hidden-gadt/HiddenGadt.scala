sealed trait Box[A]
case class IntBox(x: Int) extends Box[Int]
class HostGadt {
  def unused[A](arg: Box[A]): A = {
    class Lost {
      val a = b
      val b = 1
    }
    arg match {
      case IntBox(x) => x
    }
  }
}
