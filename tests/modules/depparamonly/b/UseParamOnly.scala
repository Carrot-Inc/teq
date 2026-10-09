package dpob

import dpoa.*

object UseParamOnly:
  def test(f: AnyRef { def run(c: Ctx)(x: c.T): Int }): Unit = ParamOnly.keep(f)
  def main(args: Array[String]): Unit =
    test(new AnyRef { def run(c: Ctx)(x: c.T): Int = 1 })
    println("paramonly")
