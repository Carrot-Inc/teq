package rbdb

import rbda.*

object UseByNameDep:
  val original: AnyRef { def run(c: Ctx)(x: => c.T): c.T } = new AnyRef { def run(c: Ctx)(x: => c.T): c.T = x }
  val returned: AnyRef { def run(c: Ctx)(x: => c.T): c.T } = ByNameDep.keep(original)
  def main(args: Array[String]): Unit = println(if returned.eq(original) then "bynamedep" else "other")
