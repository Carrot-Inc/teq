// expect: 69:18: error: forward reference to helper$default$3 (defined on line 70) extends over the definition of r (on line 69)
// expect: 74:20: error: forward reference to later (defined on line 77) extends over the definition of held (on line 76)
// expect: 81:20: error: forward reference to later (defined on line 83) extends over the definition of held (on line 82)
// expect: 87:36: error: forward reference to later (defined on line 90) extends over the definition of held (on line 89)
// expect: 94:25: error: forward reference to later (defined on line 97) extends over the definition of held (on line 96)
// expect: 101:20: error: forward reference to later (defined on line 104) extends over the definition of held (on line 103)
// expect: 108:30: error: forward reference to later (defined on line 110) extends over the definition of held (on line 109)
// expect: 113:11: error: forward reference to Base (defined on line 115) extends over the definition of held (on line 114)
// expect: 120:27: error: forward reference to given_Scope (defined on line 122) extends over the definition of i (on line 121)
// expect: 128:23: error: forward reference to Reader (defined on line 130) extends over the definition of held (on line 129)
// expect: 134:23: error: forward reference to Short (defined on line 136) extends over the definition of held (on line 135)
// expect: 142:24: error: forward reference to second (defined on line 145) extends over the definition of held (on line 144)
// expect: 148:19: error: forward reference to Later (defined on line 150) extends over the definition of held (on line 149)
// expect: 153:23: error: forward reference to Later (defined on line 155) extends over the definition of held (on line 154)
// expect: 158:24: error: forward reference to doubled (defined on line 160) extends over the definition of held (on line 159)
// expect: 163:24: error: forward reference to +++ (defined on line 165) extends over the definition of held (on line 164)
// expect: 168:22: error: forward reference to given_Doubles_Int (defined on line 170) extends over the definition of held (on line 169)
// expect: 173:41: error: forward reference to verify (defined on line 175) extends over the definition of held (on line 174)
// expect: 178:34: error: forward reference to second (defined on line 180) extends over the definition of held (on line 179)
// expect: 184:48: error: forward reference to Short (defined on line 186) extends over the definition of held (on line 185)
// expect: 190:48: error: forward reference to Later (defined on line 192) extends over the definition of held (on line 191)
// expect: 196:26: error: forward reference to Later (defined on line 198) extends over the definition of held (on line 197)
// expect: 201:28: error: forward reference to a extends over the definition of a (on line 201)
// expect: 202:33: error: forward reference to b extends over the definition of b (on line 202)
// expect: 203:38: error: forward reference to c extends over the definition of c (on line 203)
// expect: 204:33: error: forward reference to d extends over the definition of d (on line 204)
// expect: 205:33: error: forward reference to e extends over the definition of e (on line 205)
// expect: 206:33: error: forward reference to g extends over the definition of g (on line 206)
// expect: 207:47: error: forward reference to h extends over the definition of h (on line 207)
// expect: 208:35: error: forward reference to u extends over the definition of u (on line 208)
// expect: 209:43: error: forward reference to k extends over the definition of k (on line 209)
// expect: 210:60: error: forward reference to m extends over the definition of m (on line 210)
// expect: 211:49: error: forward reference to p extends over the definition of p (on line 211)
// expect: 212:44: error: forward reference to q extends over the definition of q (on line 212)
// expect: 213:33: error: forward reference to r extends over the definition of r (on line 213)
// expect: 214:33: error: forward reference to s extends over the definition of s (on line 214)
// expect: 218:26: error: forward reference to Later (defined on line 220) extends over the definition of held (on line 219)
// expect: 223:26: error: forward reference to Later (defined on line 226) extends over the definition of held (on line 224)
// expect: 229:44: error: forward reference to Later (defined on line 231) extends over the definition of held (on line 230)
// expect: 234:62: error: forward reference to Later (defined on line 236) extends over the definition of held (on line 235)
// expect: 239:44: error: forward reference to Later (defined on line 241) extends over the definition of held (on line 240)
// expect: 244:44: error: forward reference to Later (defined on line 246) extends over the definition of held (on line 245)
// expect: 249:35: error: forward reference to late (defined on line 251) extends over the definition of held (on line 250)
// expect: 257:40: error: forward reference to Mid (defined on line 256) extends over the definition of got (on line 255)
// expect: 44 errors found
// What scalac's forward-reference check names, and where, when a statement holds more than one thing it could
// tell: a default's getter stands after its method, the last one applied the farthest; a class's body is read in
// the source's order, its parameters' defaults after it (their getters are its companion's); a parent constructed
// through a trait is told at the class's name; a creation names the class, not what the class reads; a creation
// written by a type alias of the block names the alias; an extension method is told at its name, a given found
// for one where the application starts; two references whose places touch are one message; the right operand of
// `&&` stays in the tree where the left one is an inline method's call, whatever that expands to; a creation's
// type is told where the type stands, whatever is written between `new` and it; an alias that is its class
// applied to its own parameters names the class; a block is read unless it is a pure tree of a constant type: one
// with a statement that is no pure definition, or whose result an ascription or an inline method's result type
// widened, or is `()` or a class literal, stays in scalac's tree; a constructor pattern of a member class of a
// later object names the object it is selected through (a typed pattern of that class names nothing); a local
// trait's member names what it reads where it stands; in an inline method's expansion a creation whose method is
// called stays a creation.
trait Scope { def v: Int }
trait Doubles[A] { extension (a: A) def twice: A }
object Switch:
  inline def isOn: Boolean = false
  inline def pass(inline x: Int): Int = x + 1
  inline def six: Int = 6
  inline def boxed(x: Int): Int = { val y: Int = 1; 6 }
object Places:
  def getter(n: Int): Int =
    val r: Int = helper(n)
    def helper(a: Int, b: Int = 2, c: Int = 3): Int = a + b + c
    r
  def bodyOrder(n: Int): Int =
    class Body:
      def a: Int = later
      val b: Int = later
    val held: Int = n
    def later: Int = n
    held + new Body().b
  def defaultAfterBody(n: Int): Int =
    class Body(x: Int = later):
      val b: Int = later
    val held: Int = n
    def later: Int = n
    held + new Body().b
  def nestedFirst(n: Int): Int =
    class Body:
      class Inner { def get: Int = later }
      val b: Int = later
    val held: Int = n
    def later: Int = n
    held + new Body().b
  def secondConstructor(n: Int): Int =
    class Body(val x: Int):
      def this() = this(later)
      val b: Int = later
    val held: Int = n
    def later: Int = n
    held + new Body().b
  def objectBody(n: Int): Int =
    object Tools:
      def a: Int = later
      val b: Int = later
    val held: Int = n
    def later: Int = n
    held + Tools.b
  def companionFirst(n: Int): Int =
    object Sized
    class Sized(val x: Int = later)
    val held: Int = n
    def later: Int = n
    held + new Sized().x
  def throughTrait(n: Int): Int =
    class Derived extends Mixed
    val held: Int = n
    class Base
    trait Mixed extends Base
    held
  def touching(n: Int): Int =
    val o: Int =
      def first: Int = use
      val i: Int = first
      given Scope with { def v: Int = n }
      i
    def helper: Int = 1
    def use(using c: Scope): Int = c.v
    o + helper
  def capturing(n: Int): Int =
    def mk: Int = new Reader().get
    val held: Int = mk
    class Reader { def get: Int = late }
    lazy val late: Int = n
    held
  def aliasLater(n: Int): Int =
    def mk: Int = new Short().v
    val held: Int = mk
    type Short = Later
    class Later { val v: Int = n }
    held
  def plainBlock(n: Int): Int =
    def first: Int =
      class Helper:
        def get: Int = second
      new Helper().get
    val held: Int = first
    def second: Int = n
    held
  def defaultProxy(n: Int): Int =
    def mk: Int = Later().v
    val held: Int = mk
    class Later(val v: Int = 5)
    held + n
  def defaultNew(n: Int): Int =
    def mk: Int = new Later().v
    val held: Int = mk
    class Later(val v: Int = 5)
    held + n
  def extensionName(n: Int): Int =
    def first: Int = n.doubled
    val held: Int = first
    extension (x: Int) def doubled: Int = x * 2
    held
  def extensionOperator(n: Int): Int =
    def first: Int = n +++ 1
    val held: Int = first
    extension (a: Int) def +++(b: Int): Int = a + b
    held
  def extensionGiven(n: Int): Int =
    def first: Int = n.twice
    val held: Int = first
    given Doubles[Int] with { extension (a: Int) def twice: Int = a * 2 }
    held
  def inlineOperand(n: Int): Int =
    def first: Boolean = Switch.isOn && verify(n)
    val held: Boolean = first
    def verify(x: Int): Boolean = x > 0
    if held then 1 else n
  def castArgument(n: Int): Int =
    def first: Int = Switch.pass(second.asInstanceOf[Int])
    val held: Int = first
    def second: Any = n
    held

  def aliasLaterAfterComment(n: Int): Int =
    def first: Int = new /* written between */ Short(n).v
    val held: Int = first
    type Short = Later
    class Later(val v: Int)
    held
  def classAfterComment(n: Int): Int =
    def first: Int = new /* written between */ Later(n).v
    val held: Int = first
    class Later(val v: Int)
    held
  def aliasOfOwnParameters(n: Int): Int =
    type Short[A] = Later[A]
    def first: Int = new Short[Int](n).v
    val held: Int = first
    class Later[A](val v: Int)
    held
  def blocksRead(n: Int): Int =
    var a: Int = { println(a); 6 }
    var b: Int = { val t: Int = b; 6 }
    var c: Int = { lazy val t: Int = c; 6 }
    var d: Int = { def f: Int = d; List(6).head }
    var e: Int = { def f: Int = e; (6: Int) }
    var g: Int = { def f: Int = g; Switch.six }
    var h: Int = { object Held { val x: Int = h }; 6 }
    val u: Unit = { def f: Unit = u; () }
    var k: Class[?] = { def f: Class[?] = k; classOf[String] }
    var m: Int = { val t: Int = List(1).head; def f: Int = m; 6 }
    var p: Int = { var t: Int = 1; def f: Int = p + t; 6 }
    var q: Int = { val t = 5; def f: Int = q; t + 1 }
    var r: Int = { def f: Int = r; if Switch.isOn then 6 else 7 }
    var s: Int = { def f: Int = s; Switch.boxed(n) }
    a + b + c + d + e + g + h + m + p + q + r + s + n + k.hashCode
  def aliasOfOwnParametersInferred(n: Int): Int =
    type Short[A] = Later[A]
    def first: Int = new Short(n).v
    val held: Int = first
    class Later[A](val v: Int)
    held
  def aliasOfOwnParametersLater(n: Int): Int =
    def first: Int = new Short(n).v
    val held: Int = first
    type Short[A] = Later[A]
    class Later[A](val v: Int)
    held
  def memberConstructorPattern(n: Int): Int =
    def isIt(x: Any): Int = x match { case Later.Inner(v) => v; case _ => 0 }
    val held: Int = isIt(n)
    object Later { case class Inner(v: Int) }
    held
  def memberPatternImported(n: Int): Int =
    def isIt(x: Any): Int = { import Later.*; x match { case Inner(v) => v; case _ => 0 } }
    val held: Int = isIt(n)
    object Later { case class Inner(v: Int) }
    held
  def memberPatternTwoDeep(n: Int): Int =
    def isIt(x: Any): Int = x match { case Later.Mid.Inner(v) => v; case _ => 0 }
    val held: Int = isIt(n)
    object Later { object Mid { case class Inner(v: Int) } }
    held
  def memberPatternThroughInstance(n: Int): Int =
    def isIt(x: Any): Int = x match { case Later.kept.Inner(v) => v; case _ => 0 }
    val held: Int = isIt(n)
    object Later { class Holder { case class Inner(v: Int) }; val kept: Holder = new Holder }
    held
  def traitMember(n: Int): Int =
    trait Source { def get: Int = late }
    val held: Int = n
    lazy val late: Int = n
    held + (new Source {}).get
  def creationKept(n: Int): Int =
    def first: Int = make(n)
    val got: Int = first
    class Mid(val v: Int) { def get: Int = v }
    inline def make(x: Int): Int = new Mid(x).get
    got
