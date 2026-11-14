// expect: 47:22: error: forward reference to second (defined on line 49) extends over the definition of kept (on line 48)
// expect: 53:29: error: forward reference to second (defined on line 53) extends over the definition of held (on line 53)
// expect: 55:22: error: forward reference to second (defined on line 57) extends over the definition of held (on line 56)
// expect: 61:22: error: forward reference to second (defined on line 63) extends over the definition of held (on line 62)
// expect: 66:22: error: forward reference to second (defined on line 69) extends over the definition of held (on line 67)
// expect: 72:18: error: forward reference to l1 (defined on line 74) extends over the definition of a (on line 73)
// expect: 75:18: error: forward reference to l2 (defined on line 77) extends over the definition of b (on line 76)
// expect: 81:28: error: forward reference to later (defined on line 82) extends over the definition of got (on line 81)
// expect: 85:30: error: forward reference to later (defined on line 86) extends over the definition of f (on line 85)
// expect: 89:45: error: forward reference to later (defined on line 90) extends over the definition of got (on line 89)
// expect: 93:33: error: forward reference to later (defined on line 95) extends over the definition of got (on line 94)
// expect: 98:22: error: forward reference to second (defined on line 100) extends over the definition of held (on line 99)
// expect: 103:22: error: forward reference to second (defined on line 105) extends over the definition of a (on line 104)
// expect: 108:25: error: forward reference to Later (defined on line 110) extends over the definition of got (on line 109)
// expect: 113:22: error: forward reference to Later (defined on line 115) extends over the definition of held (on line 114)
// expect: 118:22: error: forward reference to Later (defined on line 120) extends over the definition of held (on line 119)
// expect: 123:43: error: forward reference to given_Shown_Int (defined on line 125) extends over the definition of held (on line 124)
// expect: 128:18: error: forward reference to x extends over the definition of x (on line 128)
// expect: 131:52: error: forward reference to f extends over the definition of f (on line 131)
// expect: 134:29: error: forward reference to second (defined on line 136) extends over the definition of held (on line 135)
// expect: 140:12: error: forward reference to Later (defined on line 143) extends over the definition of held (on line 142)
// expect: 146:27: error: forward reference to Later (defined on line 148) extends over the definition of held (on line 147)
// expect: 151:24: error: forward reference to second (defined on line 153) extends over the definition of held (on line 152)
// expect: 156:58: error: forward reference to odd (defined on line 158) extends over the definition of held (on line 157)
// expect: 161:23: error: forward reference to Holder (defined on line 163) extends over the definition of result (on line 162)
// expect: 166:23: error: forward reference to Holder (defined on line 168) extends over the definition of held (on line 167)
// expect: 171:25: error: forward reference to holder (defined on line 173) extends over the definition of got (on line 172)
// expect: 176:25: error: forward reference to late (defined on line 178) extends over the definition of got (on line 177)
// expect: 181:21: error: forward reference to helper$default$2 (defined on line 182) extends over the definition of r (on line 181)
// expect: 187:27: error: forward reference to a (defined on line 186) extends over the definition of r (on line 185)
// expect: 190:32: error: forward reference to second (defined on line 192) extends over the definition of held (on line 191)
// expect: 195:29: error: forward reference to later (defined on line 197) extends over the definition of held (on line 196)
// expect: 200:50: error: forward reference to later (defined on line 202) extends over the definition of held (on line 201)
// expect: 206:24: error: forward reference to second (defined on line 208) extends over the definition of held (on line 207)
// expect: 34 errors found
// scalac's forward-reference check (`ForwardDepChecks`), one shape a method: a value of a block defined at or
// before a definition of that block which a reference standing at or before the value names. Each error is told
// at the reference: an identifier, a creation's created type, a parent's type, a pattern's constructor, the end
// of what a searched given completes, the call a default's getter is applied in, an inline method's body where an
// expansion brought the reference. An inline method's own forward reference is one error, at its definition.
import scala.compiletime.summonInline
trait Shown[A] { def show(a: A): String }
class Wrap[A](val held: Int)
object Lib:
  inline def twice(inline x: Int): Int = x + x
  inline def own(x: Int): Int =
    def first: Int = second
    val kept: Int = first
    def second: Int = x
    kept
class Made(val kept: Int):
  def this(a: Int, b: Int) =
    this({ def first: Int = second; val held: Int = first; def second: Int = a + b; held })
  val field: Int =
    def first: Int = second
    val held: Int = first
    def second: Int = kept
    held
object Shapes:
  def method(n: Int): Int =
    def first: Int = second
    val held: Int = first
    def second: Int = n
    held
  def twoValues(n: Int): Int =
    def first: Int = second
    val held: Int = first
    val more: Int = held
    def second: Int = n
    more
  def twoLevels(n: Int): Int =
    def f: Int = l1
    val a: Int = 1
    def l1: Int = 0
    def g: Int = l2
    val b: Int = 2
    def l2: Int = 0
    f + g + a + b + n
  def byName(n: Int): Int =
    def twiceOf(x: => Int): Int = x + x
    val got: Int = twiceOf(later)
    def later: Int = n
    got
  def lambda(n: Int): Int =
    val f: () => Int = () => later
    def later: Int = n
    f()
  def nested(n: Int): Int =
    val got: Int = { val inner = 1; inner + later }
    def later: Int = n
    got
  def classBody(n: Int): Int =
    class Use { def read: Int = later }
    val got: Int = new Use().read
    def later: Int = n
    got
  def variable(n: Int): Int =
    def first: Int = second
    var held: Int = first
    def second: Int = n
    held
  def tuple(n: Int): Int =
    def first: Int = second
    val (a, b) = (first, 1)
    def second: Int = n
    a + b
  def created(n: Int): Int =
    def make: Int = new Later(n).v
    val got: Int = make
    class Later(val v: Int)
    got
  def caseClass(n: Int): Int =
    def first: Int = Later(n).v
    val held: Int = first
    case class Later(v: Int)
    held
  def module(n: Int): Int =
    def first: Int = Later.value
    val held: Int = first
    object Later { val value: Int = n }
    held
  def searched(n: Int): String =
    def first: String = summon[Shown[Int]].show(n)
    val held: String = first
    given Shown[Int] with { def show(a: Int): String = "int" + a }
    held
  def itself(n: Int): Int =
    val x: Int = x + n
    x
  def recursive(n: Int): Int =
    val f: Int => Int = i => if i <= 0 then 0 else f(i - 1)
    f(n)
  def eta(n: Int): Int =
    def first: Int => Int = second
    val held: Int = first(1)
    def second(x: Int): Int = x + n
    held
  def unapplied(n: Int): Int =
    def first(x: Any): Int = x match
      case Later(v) => v
      case _ => n
    val held: Int = first(n)
    case class Later(v: Int)
    held
  def anonymous(n: Int): Int =
    def first: Int = (new Later(n) {}).v
    val held: Int = first
    class Later(val v: Int)
    held
  def defaulted(n: Int): Int =
    def first(x: Int = second): Int = x
    val held: Int = first()
    def second: Int = n
    held
  def mutual(n: Int): Int =
    def even(i: Int): Boolean = if i == 0 then true else odd(i - 1)
    val held: Boolean = even(n)
    def odd(i: Int): Boolean = if i == 0 then false else even(i - 1)
    if held then 1 else 0
  def parent(n: Int): Int =
    class Use extends Holder[Int](n)
    val result: Int = new Use().value
    class Holder[A](val value: A)
    result
  def parentPlain(n: Int): Int =
    class Use extends Holder
    val held: Int = new Use().value + n
    class Holder { val value: Int = 1 }
    held
  def prefix(n: Int): Int =
    def make: Int = new holder.Inner(n).value
    val got: Int = make
    object holder { class Inner(val value: Int) }
    got
  def typeArgument(n: Int): Int =
    def make: Int = new Wrap[late.type](n).held
    val got: Int = make
    object late
    got
  def getter(n: Int): String =
    val r: String = helper(n)
    inline def helper(a: Int, suffix: String = "."): String = a.toString + suffix
    r + helper(n)
  def expansion(n: Int): Int =
    val r: Int = f()
    def a: Int = n
    inline def f(): Int = a
    r
  def argument(n: Int): Int =
    def first: Int = Lib.twice(second)
    val held: Int = first
    def second: Int = n
    held
  def inlineBody(n: Int): Int =
    inline def first: Int = later + 1
    val held: Int = n
    inline def later: Int = n
    held + first
  def forValue(n: Int): Int =
    def first: List[Int] = for x <- List(n); y = later(x) yield y
    val held: List[Int] = first
    def later(x: Int): Int = x + 1
    held.head
  def caseLambda(n: Int): Int =
    val f: ((Int, Int)) => Int = { case (a, b) =>
      def first: Int = second
      val held: Int = first
      def second: Int = a + b
      held
    }
    f((n, 1))
  def main(args: Array[String]): Unit = println(method(1) + Lib.own(2))
