package p

class B extends Has:
  val n: Int = 2
  val x: Int = readA

class BMaker extends BF:
  def make(): Has = new B

inline implicit def bf: BF = new BMaker
