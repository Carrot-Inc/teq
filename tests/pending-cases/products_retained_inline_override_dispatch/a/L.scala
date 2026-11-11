package s4m

trait A:
  def m: Int = 1

trait B extends A:
  override inline def m: Int = 2

class E extends B
