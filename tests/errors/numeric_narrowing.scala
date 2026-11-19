// expect: 8:17: error: Found:    300⏎Required: Byte
// expect: 9:18: error: Found:    1.5⏎Required: Float
// expect: 10:18: error: Found:    Byte⏎Required: Char
// expect: 11:18: error: Found:    Null⏎Required: Short
// expect: operator & cannot be applied to Float
// expect: operator << cannot be applied to Float
@main def run(): Unit =
  val b: Byte = 300
  val f: Float = 1.5d
  val c: Char = (1: Byte)
  val s: Short = null
  val m = 1.5f & 1
  val sh = 1 << 2.5f
  println(b)
