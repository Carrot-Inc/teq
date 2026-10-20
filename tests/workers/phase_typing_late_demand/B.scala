object B:
  val use = Later.bad
  val dependent = A.inferred
object Later:
  val bad = { val err: String = 123; 1 }
