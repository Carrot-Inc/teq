// expect: b_cycle.scala:4:10: error: cyclic export of B, which exports A
// The tables of the cycle are first asked for while `Y`'s header is completed, through the
// import; the cycle is reported once whatever the order of the files.
import cyc.A.*

class Y extends Z:
  def y: Int = 1

class Z
