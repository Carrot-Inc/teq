// `widenEnumCase` (Applications.scala:1689-1704): a synthetic enum case `apply` or `copy` widens to the intersection of
// the case type's parents read through its prefix, its transparent traits dropped, where that is within the
// application's prototype; a `new` never widens, and nor does an application dotty lifts into a block (a `copy` whose
// default reads a receiver that is no pure path, Applications.scala:335).
class O:
  trait Tr:
    def tag: String = "tr"
  enum E:
    case K(n: Int) extends E with Tr

transparent trait Tt
trait Tr2
enum F:
  case K(n: Int) extends F with Tt
  case R(n: Int) extends F with Tr2 with Tt
  case P(n: Int)

enum G:
  case K(n: Int) extends G with Tr2

enum H:
  case M[A](a: A) extends H

enum J:
  case K(n: Int)
  case Q(a: Int, b: Int)

object HJ:
  val q: J.Q = J.Q(3, 4)

object Main:
  def f(e: F): String = "F"
  @annotation.targetName("f2") def f(e: F & Tt): String = "F&Tt"
  def g(e: F): String = "F"
  @annotation.targetName("g2") def g(e: F & Tr2): String = "F&Tr2"
  def h(e: G): String = "G"
  def h(e: G.K): String = "K"
  def w(x: J): String = "J"
  def w(x: J.K): String = "K"
  def w(x: J.Q): String = "Q"
  def mk(): J.Q = J.Q(1, 2)
  def main(args: Array[String]): Unit =
    val a = new O
    val k = a.E.K(1)
    val t: a.Tr = k
    val m: H = H.M("x")
    val ks: List[a.E & a.Tr] = List(a.E.K(1), a.E.K(2))
    println((t.tag, k, m, ks))
    println((f(F.K(1)), g(F.R(2)), f(F.R(3))))
    val n = new F.K(4)
    val nk: F.K = n
    val u: F.P | Int = F.P(5)
    val o: Option[F.P] = Some(F.P(6))
    println((nk.n, u, o, F.P(7).n))
    val gk: G.K = G.K(8)
    val c = gk.copy(n = 9)
    val d: G.K = gk.copy(n = 10)
    println((h(c), h(d), h(G.K(11)), h(new G.K(12))))
    println(List(w(J.Q(1, 2).copy(b = 5)), w(J.K(1).copy()), w(mk().copy(a = 7)), w(HJ.q.copy(a = 8)), w(J.K(2).copy(3)), w(J.Q(1, 2).copy(4, 5))).mkString(" "))
    val jk = J.K
    println(List(w(J.K.apply(3)), w(jk.apply(5)), w(J.K(6))).mkString(" "))
