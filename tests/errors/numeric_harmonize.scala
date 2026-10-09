// Numeric widening and the harmonisation of Int literals in branches and varargs.
// expect: 28:21: error: type mismatch: found Long, required Int
// expect: 29:22: error: type mismatch: found Double, required Long
// expect: 35:19: error: type mismatch: found Int | Long, required Long
// expect: 37:19: error: type mismatch: found Int | Long, required Long
// expect: 43:21: error: type mismatch: found Int | Double, required Double
// expect: 45:21: error: type mismatch: found Long | Double, required Double
// expect: 54:25: error: type mismatch: found List[Int | Long], required List[Long]
// expect: 56:25: error: type mismatch: found List[Int | Long], required List[Long]
// expect: 60:24: error: type mismatch: found List[Char], required List[Int]
// expect: 73:16: error: number too large for Int; use an L suffix for Long
// expect: 76:19: error: type mismatch: found Int | Long, required Long
// expect: 78:19: error: type mismatch: found Int | Long, required Long
object N:
  val c = true
  val i: Int = 3
  val l: Long = 4L
  val d: Double = 2.5
  val ch: Char = 'a'
  def takesLong(x: Long): Long = x
  def takesDouble(x: Double): Double = x
  def takesInt(x: Int): Int = x
  val a1 = takesLong(1)
  val a2 = takesLong(i)            // ok: Int widens to Long
  val a3 = takesDouble(i)          // ok
  val a4 = takesDouble(l)          // ok
  val a5 = takesInt(ch)            // ok: Char widens to Int
  val a6 = takesInt(l)             // scalac: error
  val a7 = takesLong(d)            // scalac: error
  val a8 = takesLong(ch)           // ok: Char widens to Long
  val a9: Long = ch                // ok
  val b1 = if c then 1 else 2L
  val b1t: Long = b1               // ok: harmonised
  val b2 = if c then i else 2L
  val b2t: Long = b2               // harmonised? (Int non-literal + Long literal)
  val b3 = if c then i else l
  val b3t: Long = b3               // scalac: error, b3: Int | Long (no literal)
  val b4 = if c then 1 else 'a'
  val b4t: Int = b4                // harmonised to Int?
  val b5 = if c then 1 else 2.5
  val b5t: Double = b5             // ok
  val b6 = if c then i else 2.5
  val b6t: Double = b6             // ?
  val b7 = if c then l else 2.5
  val b7t: Double = b7             // ?
  val b8 = c match
    case true => 1
    case false => 2L
  val b8t: Long = b8               // ok
  val b9 = if c then 1 else "a"
  val l1 = List(1, 2L)
  val l1t: List[Long] = l1         // ok
  val l2 = List(i, 2L)
  val l2t: List[Long] = l2         // ?
  val l3 = List(i, l)
  val l3t: List[Long] = l3         // scalac: error (Int | Long or AnyVal)
  val l4 = List(1, 2.5)
  val l4t: List[Double] = l4       // ok
  val l5 = List('a', 1)
  val l5t: List[Int] = l5          // ?
  val mx = math.max(1L, 2)
  val mxt: Long = mx               // ok
  val mn = math.min(i, 2.5)
  val mnt: Double = mn             // ok
  val sum = i + l
  val sumt: Long = sum             // ok
  val sum2 = ch + 1
  val sum2t: Int = sum2            // ok
  val neg = -ch
  val negt: Int = neg              // ok
  val big = 1 << 40
  val lit: Long = 2147483648L
  val tooBig = 2147483648          // scalac: error (number too large)
  def gen[T](a: T, b: T): T = a
  val g1 = gen(1, 2L)
  val g1t: Long = g1               // scalac: harmonised? (type variable, not if)
  val g2 = gen(i, 2L)
  val g2t: Long = g2               // ?
  val arr = Array(1, 2L)
  val arrt: Array[Long] = arr      // ?

@main def main(): Unit =
  println(N.b1)
  println(N.l1)
  println(N.mx)
