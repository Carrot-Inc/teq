package ctr

class P3 private[ctr] (val x: Int)
class P4 protected[ctr] (val x: Int)
class Prot protected (val x: Int)

class S(val x: Int):
  private[ctr] def this() = this(7)
  protected def this(s: String) = this(s.length)

case class K protected (x: Int)
