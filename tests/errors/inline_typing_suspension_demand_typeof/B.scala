object B:
  type Use = Later.bad.type
  val dependent = A.inferred
object Later:
  val bad = { val err: String = 123; 1 }
