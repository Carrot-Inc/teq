// A Java field is a constant only as the reference reads one: static and final, its initializer one literal followed
// by `;`, a String only where its type is written `String`, converted to the written type where it can be; no folding.
// expect: Use.scala:13:16: error: Found:    Int⏎Required: 2
// expect: Use.scala:14:17: error: Found:    Int⏎Required: 2
// expect: Use.scala:15:24: error: Found:    String⏎Required: "q"
// expect: Use.scala:16:18: error: Found:    Int⏎Required: 1
// expect: Use.scala:17:21: error: Found:    Int⏎Required: 3
// expect: Use.scala:18:21: error: Found:    Int⏎Required: 4
object Use {
  val one: 1 = J.ONE
  val text: "t" = J.TEXT
  val second: 2 = J.SECOND
  val sum: 2 = J.SUM
  val cast: 2 = J.CAST
  val qualified: "q" = J.QUALIFIED
  val first: 1 = J.FIRST
  val instance: 3 = new J().instance
  val nonFinal: 4 = J.nonFinal
  val tooBig: Byte = J.TOO_BIG
}
