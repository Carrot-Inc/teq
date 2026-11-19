// Literal types: an inferred literal type widens to its class, an ascribed one is kept.
// expect: 17:14: error: Found:    Int⏎Required: 1
// expect: 19:16: error: Found:    String⏎Required: "x"
// expect: 20:16: error: Found:    2⏎Required: 1
// expect: 24:14: error: Found:    Int⏎Required: 1
// expect: 26:20: error: Found:    List[Int]⏎Required: List[1]
// expect: 31:14: error: Found:    Int⏎Required: 1
// expect: 33:29: error: Found:    "medium"⏎Required: "fast" | "slow"
// expect: 35:20: error: Found:    'b'⏎Required: 'a'
// expect: 37:21: error: Found:    false⏎Required: true
object Lits:
  final val a = 1
  val b = 1
  def f = 1
  val c: 1 = 1
  val d = c
  val e: 1 = d               // scalac: error (d: Int)
  val s = "x"
  val t: "x" = s             // scalac: error
  val bad: 1 = 2             // scalac: error
  def id[T](x: T): T = x
  val g = id(c)              // T widened to Int
  val h: 1 = id(c)           // ok: expected type keeps T = 1
  val k: 1 = g               // scalac: error
  val l = List(c)            // List[Int]
  val m: List[1] = l         // scalac: error
  val n: List[1] = List(c)   // ok
  def sing[T <: Singleton](x: T): T = x
  val p: 1 = sing(c)         // ok
  val q = sing(c)            // q: 1 (Singleton bound keeps it)
  val r: 1 = q               // ok
  val w: "fast" | "slow" = "fast"
  val w2: "fast" | "slow" = "medium"   // scalac: error
  val ch: 'a' = 'a'
  val chBad: 'a' = 'b'       // scalac: error
  val bo: true = true
  val boBad: true = false    // scalac: error

@main def main(): Unit =
  println(Lits.d + Lits.g)
