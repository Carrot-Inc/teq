// expect: type mismatch: found 300, required Byte
// expect: type mismatch: found 1.5, required Float
// expect: type mismatch: found Byte, required Char
// expect: type mismatch: found Null, required Short
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
