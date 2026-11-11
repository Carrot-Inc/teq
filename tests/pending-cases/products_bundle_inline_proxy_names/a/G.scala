package pxa

class Own:
  override def toString: String = "own"

object G:
  transparent inline def tr(y: Own): String = y.toString
  inline def il(y: Own): String = y.toString
