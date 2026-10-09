package dcb

import dca.*

object UseCross:
  def test(f: AnyRef { def run(c: Ctx)(x: c.T): c.T }): AnyRef { def run(c: Ctx)(x: c.T): c.T } = Cross.keep(f)
  def main(args: Array[String]): Unit =
    val f: AnyRef { def run(c: Ctx)(x: c.T): c.T } = new AnyRef { def run(c: Ctx)(x: c.T): c.T = x }
    println(if test(f).eq(f) then "cross" else "other")
