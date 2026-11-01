// An enum's body reading its case's parameters while the enum is constructed, through a member or as its arguments:
// the case's fields are written before the enum's `$init` runs (dotty's `Constructors`: `copyParams` before the
// superclass's constructor call), a secondary constructor's chain and a `using` list included.
enum E:
  def x: Int
  val y = x * 2
  case C(x: Int)

enum F(val k: Int):
  def x: Int
  val y = x * k
  case G(x: Int) extends F(10)

enum U(using val k: Int):
  def x: Int
  val y = x + k
  case V(x: Int) extends U(using 5)

enum S(val a: Int):
  def this() = this(9)
  def x: Int
  val y = a + x
  case C(x: Int) extends S()

enum L(val s: Int):
  case G(a: Int)(b: Int) extends L(a + b)
  case H(a: Int)(using b: Int) extends L(b)

enum P:
  println("made " + this)
  case Q(n: Int)

@main def run(): Unit =
  println(E.C(4).y)
  println(F.G(4).y)
  println(U.V(1).y)
  println(S.C(1).y)
  println(L.G(1)(2).s)
  given Int = 7
  println(L.H(1).s)
  P.Q(5)
