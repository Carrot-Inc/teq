class Target

object F:
  def fresh: Unit = Rep.warnAt[Target]("fresh, expanded at its unit")
  def held: Int = Rep.same(Rep.warnAtValue[Target]("held, expanded at typing"))
