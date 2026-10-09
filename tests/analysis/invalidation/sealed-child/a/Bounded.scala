package sca

object Bounded:
  def name[T <: Shape](s: T): String = s match
    case _: Circle => "c"
    case _ => "o"
