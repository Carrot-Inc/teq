// Sixteen objects, a file each, of inline methods whose results are inferred: each caller calls a callee of
// another file (the next file's for two of its three, the one before's for the third), so that a definition's
// check is first needed inside another's, and by several workers at once. Expects the sixteen totals and a
// sum; scalac prints 15 27 39 .. 195 and 376.
@main def run(): Unit =
  val parts = List(Lib00.total(1), Lib01.total(2), Lib02.total(3), Lib03.total(4), Lib04.total(5), Lib05.total(6), Lib06.total(7), Lib07.total(8), Lib08.total(9), Lib09.total(10), Lib10.total(11), Lib11.total(12), Lib12.total(13), Lib13.total(14), Lib14.total(15), Lib15.total(16))
  println(parts.mkString(" "))
  println(Lib00.outer0(0) + Lib01.outer0(1) + Lib02.outer0(2) + Lib03.outer0(3) + Lib04.outer0(4) + Lib05.outer0(5) + Lib06.outer0(6) + Lib07.outer0(7) + Lib08.outer0(8) + Lib09.outer0(9) + Lib10.outer0(10) + Lib11.outer0(11) + Lib12.outer0(12) + Lib13.outer0(13) + Lib14.outer0(14) + Lib15.outer0(15))
