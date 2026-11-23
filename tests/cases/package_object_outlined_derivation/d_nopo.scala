package oq

sealed trait QS
case class QA(i: Int) extends QS
case class QB(s: String) extends QS
case object QC extends QS
