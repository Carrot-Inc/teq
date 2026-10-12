// A literal has its constant type (dotty's `TypeAssigner.assignType(Literal)`), which a `final val`
// keeps and an inferred type widens where its variable is instantiated (`widenInferred`): every
// kind of constant, a `Float`'s too, through `final val`s, generic applications, tuples and
// operations over constants (`ConstFold`).
object M:
  inline def one: 1 = 1
object C:
  final val i = 1
  final val l = 1L
  final val d = 1.5
  final val f = 1.5f
  final val c = 'a'
  final val b = true
  final val s = "a"
  final val r = 16777217f
  final val sum = 1 + 2
  final val cat = "a" + "b"
  final val fsum = 1.5f + 1
  final def id = 7
def id[T](x: T): T = x
def pair[A, B](a: A, b: B): (A, B) = (a, b)
@main def run() =
  val i: 1 = C.i
  val l: 1L = C.l
  val d: 1.5 = C.d
  val f: 1.5f = C.f
  val c: 'a' = C.c
  val b: true = C.b
  val s: "a" = C.s
  val r: 16777216f = C.r
  val sum: 3 = C.sum
  val cat: "ab" = C.cat
  val fsum: 2.5f = C.fsum
  val neg: -1 = -M.one
  val lt: true = 1 < 2
  val widened: 3L = 1 + 2L
  println((i, l, d == 1.5, f == 1.5f, c, b, s, r == 16777216f, sum, cat, fsum == 2.5f, neg, lt, widened))
  // Widened where a variable is instantiated or a value inferred.
  val x = 1
  var y = 2
  y = 3
  val xs = List(1, 2)
  val t = (1, "a")
  val u = id(1)
  val p = pair(1.5f, 'c')
  val n: Int = C.id
  val tt: (Int, String) = t
  val pp: (Float, Char) = p
  println((x + y, xs.sum, t, u + 1, pp._1 == 1.5f, pp._2, n))
  // Kept where the expected type asks for the constant.
  val kept: (1, "a") = (1, "a")
  val kept2: List[1] = List(1)
  val kept3: 1 = id(1)
  println((kept, kept2, kept3))
  // Harmonised: an Int constant among one other numeric class takes that class.
  val h = if x > 0 then 1 else 2.5
  val hs = List(1, 2L)
  val hc = List(M.one, 0.5)
  println(((h * 2).toInt, hs, (hc.sum * 2).toInt, (Math.max(5, 3.5) * 4).toInt))
  // A `Float` constant read through `Any` is a `Float`, as its field and its erasure are.
  val anyF: Any = C.f
  val anyG: Any = C.fsum
  println((anyF.isInstanceOf[Float], anyG.isInstanceOf[Float], id(C.f) == 1.5f, List(C.f, C.fsum).sum == 4f))
