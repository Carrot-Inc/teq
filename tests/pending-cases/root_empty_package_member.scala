// A class of the empty package selected through `_root_` (`_root_.EK`): scalac 3.8.4 rejects it (the .expected), "type
// EK is not a member of <root>", the root and the empty package being two. teq accepts it and prints 4: its one package
// id for both resolves the selection. Since b063a846d8 writes a kept tree's `_root_` as the root package, scalac's
// reader of teq's pickle of the selection, in an inline body read downstream, fails on it ("Found: _root_.type
// Required: ?{ EK: ? }"), where `<empty>.EK` read before. Found by reviewer #14 of the workers exports helper (push 9).
class EK(val n: Int)

object EU:
  def e(x: EK): Int = (x: _root_.EK).n

@main def run(): Unit =
  println(EU.e(EK(4)))
