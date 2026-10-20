object B:
  val use = Later.get()
  val dependent = A.inferred
object Later:
  def get(x: Int = { val err: String = 123; 1 }): Int = x
