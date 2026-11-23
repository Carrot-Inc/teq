package op.po

sealed trait TS
case class TA(i: Int) extends TS
case class TB(s: String) extends TS
case object TC extends TS
