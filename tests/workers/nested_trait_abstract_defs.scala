// A trait of abstract defs alone, nested in a generic trait, mixed in by a class whose instances an
// object extending the outer trait makes in its vals: the class's check asks whether the nested
// trait is a pure interface while other workers complete those defs.
trait Catalog[A]:
  trait Entry:
    self: A =>
    def label: String
    def rank: Int

object Tools extends Catalog[Tool]:
  val hammer = new Tool(0, "hammer") {}
  val saw = new Tool(1, "saw") {}

sealed abstract class Tool(val rank: Int, val label: String) extends Tools.Entry

@main def run(): Unit =
  println(Tools.hammer.label + Tools.saw.rank)
