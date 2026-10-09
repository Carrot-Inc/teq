package dspb

import dspa.*

object UseSameParamOnly:
  def test(f: AnyRef { def run(c: Ctx, x: c.T): Int }): Unit = SameParamOnly.keep(f)
  def main(args: Array[String]): Unit =
    test(new AnyRef { def run(c: Ctx, x: c.T): Int = 1 })
    println("sameparamonly")
