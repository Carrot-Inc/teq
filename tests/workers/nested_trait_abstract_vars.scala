// As nested_trait_abstract_defs.scala, the nested trait's members abstract vars: each has a
// setter, a def of one parameter whose type is the var's.
trait Catalog[A]:
  trait Entry:
    self: A =>
    var label: String
    var rank: Int

object Tools extends Catalog[Tool]:
  val hammer = new Tool(0, "hammer") {}
  val saw = new Tool(1, "saw") {}

sealed abstract class Tool(var rank: Int, var label: String) extends Tools.Entry

@main def run(): Unit =
  println(Tools.hammer.label + Tools.saw.rank)
