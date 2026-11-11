package bxa

object Lib:
  def boxed[A <: scala.runtime.BoxedUnit](x: Any): A = x.asInstanceOf[A]
