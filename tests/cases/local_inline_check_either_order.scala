// A block's inline method gives the same program whether the block reaches its definition first or a call does:
// called after its definition inside a refining case, reading the enclosing method's type parameters, calling itself
// through an inline condition, called only inside a function; a member inline method first called inside a case that
// refines its class's type parameter; a body that summons for the type parameter, matches on it, or has its result
// inferred; a nearer given around the call, an overloaded call around it, a wider expected type at it. And what
// stands round the DEFINITION is its own: defined inside a refining case after a value and a given of that case, the
// body takes the case's bound, the value and the given; defined inside a given's initialiser, its search leaves
// that given out, as scalac's does for a search in the given's own right-hand side.
sealed trait Box[A]
case class I(value: Int) extends Box[Int]
case class S(value: String) extends Box[String]

trait Shown[T]:
  def show(t: T): String
given Shown[Int] with
  def show(t: Int): String = "int " + (t + 1)

trait Marked:
  def name: String

sealed trait Tag[A]
case object IntTag extends Tag[Int]
case object StrTag extends Tag[String]

class Cell[A](val tag: Tag[A], val value: A):
  def which(x: Any): String = "any"
  def which(x: Int): String = "int " + (x + 1)
  def first: String = tag match
    case IntTag => show(value)
    case _ => "other"
  inline def show(a: A): String = which(a)
  def second: String = show(value)

object Orders:
  def which(x: Any): String = "any"
  def which(x: Int): String = "int"
  def after[A](box: Box[A], x: A): String =
    inline def helper(a: A): String = which(a)
    val inside = box match
      case I(_) => helper(x)
      case _ => "other"
    inside + ", " + helper(x)
  def typeParams[A, B](a: A, b: B): String =
    val first = pair(a)
    inline def pair(x: A): String = which(x) + "/" + which(b)
    first
  def recursive(n: Int): Int =
    val r = count(3)
    inline def count(inline k: Int): Int = if k == 0 then n else 1 + count(k - 1)
    r
  def inFunction(xs: List[Int]): List[String] =
    val mapped = xs.map(x => label(x))
    inline def label(k: Int): String = which(k) + k
    mapped
  def summoned[A](box: Box[A], x: A)(using fallback: Shown[A]): String =
    val inside = box match
      case I(_) => helper(x)
      case _ => "other"
    inline def helper(a: A): String = summon[Shown[A]].show(a)
    inside + ", " + helper(x)
  def matched[A](box: Box[A], other: Box[A]): String =
    val inside = box match
      case I(_) => helper(other)
      case _ => "other"
    inline def helper(b: Box[A]): String = b match
      case I(n) => "int " + n
      case S(s) => "string " + s
    inside + ", " + helper(other)
  def inferred: String =
    def early = helper + "!"
    inline def helper = which(1) + which("s")
    early + " " + helper
  def nearer: String =
    given outer: Int = 1
    val inner = {
      given nearest: Int = 2
      helper
    }
    inline def helper: String = "found " + summon[Int]
    inner + ", " + helper
  def pick(f: Int => Int, n: Int): Int = f(1)
  def pick(f: Int => Int, n: String): Int = f(2)
  def overloaded: Int = pick(x => {
    val early = helper
    inline def helper: Int = x + 10
    early
  }, "b")
  def wider: String =
    val wide: Long = helper
    inline def helper = 1
    which(helper) + " " + wide
  def bumped(x: Int): String = "int " + (x + 1)
  def bumped(x: Any): String = "any"
  def insideCase[A](box: Box[A], x: A): String = box match
    case I(_) =>
      val earlier = 10
      given String = "given"
      val first = helper
      inline def helper: String = bumped(x) + "/" + earlier + "/" + summon[String]
      first
    case _ => "other"
  given farther: Marked with
    def name: String = "farther"
  def wrap(m: => Marked): Marked = new Marked:
    def name: String = if m eq this then "nearer" else m.name
  def insideGiven: String =
    given nearer: Marked = {
      val first = helper
      inline def helper: Marked = wrap(summon[Marked])
      first
    }
    nearer.name

@main def run(): Unit =
  println(Orders.after(I(1), 2))
  println(Orders.after(S("a"), "b"))
  println(Orders.typeParams(1, "x"))
  println(Orders.recursive(4))
  println(Orders.inFunction(List(1, 2)))
  given Shown[String] with
    def show(t: String): String = "string " + t
  val viaFallback = new Shown[Int] { def show(t: Int): String = "fallback " + t }
  println(Orders.summoned(I(1), 2)(using viaFallback))
  println(Orders.summoned(S("a"), "b"))
  println(Orders.matched(I(1), I(2)))
  println(Orders.matched(S("a"), S("b")))
  println(Orders.inferred)
  println(Orders.nearer)
  println(Orders.overloaded)
  println(Orders.wider)
  println(Orders.insideCase(I(1), 2))
  println(Orders.insideGiven)
  println(new Cell(IntTag, 1).first + ", " + new Cell(IntTag, 1).second)
  println(new Cell(StrTag, "b").first + ", " + new Cell(StrTag, "b").second)
