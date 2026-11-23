package op

package object po:
  sealed trait PS
  case class PA(i: Int) extends PS
  case class PB(s: String) extends PS
  case object PC extends PS
