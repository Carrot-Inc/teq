class Target1
class Target2

object F:
  def u1: Unit = Rep.warnAt[Target1]("B1")
  def u2: Int = Run.viaTwice(2)
  def u3: Unit = { Rep.warnAt[Target1]("B1"); Rep.warnAt[Target2]("B2") }
