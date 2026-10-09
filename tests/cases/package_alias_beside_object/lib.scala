package pz

object Trace:
  def empty: Trace = "none"

type Trace = String
type UIO[A] = Option[A]

object Duration:
  val Zero: Duration = 0L
type Duration = Long
