package p

class A extends Has:
  val n: Int = 1
  val x: Int = readB

class AMaker extends AF:
  def make(): Has = new A

inline implicit def af: AF = new AMaker
