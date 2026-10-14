// expect: b_cycle.scala:4:12: error: Conflicting definitions:
// The forwarders of the cycle are first asked for while `Y`'s header is completed, through the
// import; the conflict they make is reported once, at the first object's export, whatever the
// order of the files, as scalac reports it.
import cyc.A.*

class Y extends Z:
  def y: Int = 1

class Z
