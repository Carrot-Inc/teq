// A class of the empty package selected
// through `_root_` (`_root_.EK`): scalac 3.8.4
// rejects it (the .expected), "type EK is not
// a member of <root>", the root and the empty
// package being two. teq is to reject it too.
class EK(val n: Int)

object EU:
  def e(x: EK): Int = (x: _root_.EK).n

@main def run(): Unit =
  println(EU.e(EK(4)))
