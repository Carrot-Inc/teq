package sa

object O:
  println("init O")
  object I
  val v: I.type = I

object Paths:
  inline def a14(): O.v.type = (O.v: O.v.type)
