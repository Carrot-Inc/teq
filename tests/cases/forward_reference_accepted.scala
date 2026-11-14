// What scalac's forward-reference check accepts in a block: a later definition named over
// nothing it tests, or by nothing its tree still holds.
trait Marked
object Constants:
  transparent inline def three: Int = 3
  transparent inline def yes: Boolean = true
  transparent inline def threeOf(x: Int): Int = { val y: Int = 1; 3 }
  inline def bodyOf(inline x: Int): Int = { def f: Int = x; 3 }
  inline def plainThree: Int = 3
object Test:
  def sized[A](n: Int): Int = n
  // A later inline method called over a value: the call is expanded, the expansion names nothing ahead.
  def inlineOver(n: Int): Int =
    val got: Int = later(1)
    val base: Int = n
    inline def later(x: Int): Int = x + 4
    got + base - n
  // A lazy value between is not tested, and one named ahead is entered ahead.
  def lazyBetween(n: Int): Int =
    def first: Int = second
    lazy val held: Int = first
    def second: Int = n
    held
  def lazyAhead(n: Int): Int =
    def first: Int = late
    lazy val late: Int = n
    first
  // No value between the reference and the definition.
  def noValue(n: Int): Int =
    def first: Int = second + 1
    def second: Int = n
    first
  // A class's members are no level.
  class Members(n: Int):
    def first: Int = second
    val held: Int = n
    def second: Int = held
  // A later class named in a type alone: a type test's, a trait parent, a method's type argument. (A typed
  // pattern's type is a reference to scalac, `case x: Later`: not among these.)
  def typeOnly(n: Int): Int =
    def first(x: Any): Int = if x.isInstanceOf[Later] then 1 else n
    val held: Int = first(n)
    class Later
    held
  def traitParent(n: Int): Int =
    class Use extends Later
    val held: Int = n
    trait Later
    if new Use().isInstanceOf[Later] then held else 0
  def typeArgument(n: Int): Int =
    def first: Int = sized[late.type](n)
    val held: Int = first
    object late
    held
  // A branch scalac folds away is not in its tree.
  def folded(n: Int): Int =
    def first: Int = if false then second else n
    val held: Int = first
    def second: Int = n + 1
    held
  // A method that names itself, and a value that is no definition.
  def recursion(n: Int): Int =
    def loop(i: Int): Int = if i <= 0 then 0 else 1 + loop(i - 1)
    val held: Int = loop(n)
    held
  def wildcard(n: Int): Int =
    def first: Int = second
    val _ = first
    def second: Int = n
    first
  // A creation written by a type alias of the block names the alias, which stands before the value.
  def aliasEarlier(n: Int): Int =
    type Short = Later
    def mk: Int = new Short().v
    val held: Int = mk
    class Later { val v: Int = n }
    held
  def aliasAnonymous(n: Int): Int =
    type Short = Later
    def mk: Int = (new Short() {}).v
    val held: Int = mk
    class Later { val v: Int = n }
    held
  def aliasOfAlias(n: Int): Int =
    type Short = Later
    type Shorter = Short
    def mk: Int = new Shorter().v
    val held: Int = mk
    class Later { val v: Int = n }
    held
  // An operand scalac folds away with its operator.
  def foldedAnd(n: Int): Int =
    def first: Boolean = false && second > 0
    val held: Boolean = first
    def second: Int = 100
    if held then 1 else n
  def foldedOr(n: Int): Int =
    def first: Boolean = true || second > 0
    val held: Boolean = first
    def second: Int = 100
    if held then n else 1
  // A parameter's default stands in the class's companion, here written after the value.
  def companionAfter(n: Int): Int =
    class Sized(val x: Int = later)
    val held: Int = n
    def later: Int = n
    object Sized
    held + new Sized().x - n
  // The alias a creation is written by is the one the typing resolved, whatever stands between `new` and it.
  def aliasAfterComment(n: Int): Int =
    type Short = Later
    def first: Int = new /* written between */ Short(n).v
    val held: Int = first
    class Later(val v: Int)
    held
  def aliasInParentheses(n: Int): Int =
    type Short = Later
    def first: Int = new (Short)(n).v
    val held: Int = first
    class Later(val v: Int)
    held
  def aliasInBackquotes(n: Int): Int =
    type `Short name` = Later
    def first: Int = new `Short name`(n).v
    val held: Int = first
    class Later(val v: Int)
    held
  def aliasAnonymousAfterComment(n: Int): Int =
    type Short = Later
    def first: Int = (new /* written between */ Short(n) {}).v
    val held: Int = first
    class Later(val v: Int)
    held
  def aliasOfAnObject(n: Int): Int =
    object held0 { type Short = Later }
    def first: Int = new held0.Short(n).v
    val held: Int = first
    class Later(val v: Int)
    held
  // A block scalac replaces by its constant holds nothing: its statements pure definitions, its result a constant.
  def foldedBlocks(n: Int): Int =
    var a: Int = { def f: Int = a; 3 }
    val b: Int = { def f: Int = b; 3 }
    var c: Int = { class C { def f: Int = c }; 3 }
    var d: Any = { def f: Any = d; 3 }
    var e: Int = if true then { def f: Int = e; 3 } else 7
    var g: Int = { val t = 5; def f: Int = g + t; 3 }
    var h: Int = { def f: Int = h; 1 + 2 }
    a + b + c + e + g + h - 15 + (if d == 3 then 0 else 1) + n - n
  // The statements of a folded block are pure by scalac's reading: a value that is no `var` bound to a pure tree
  // (a stable value, a function, a creation of a class that initialises nothing, a lazy value's own), an object
  // and a class; its result a constant as typed, an inner such block and an `if` on a constant read first.
  def foldedByPurity(n: Int): Int =
    class Empty
    val k: Int = n
    var a: Int = { val t: Int = k; def f: Int = a + t; 3 }
    var b: Int = { val g: Int => Int = x => x + b; 3 }
    val c: Int = { lazy val t: Int = c; 3 }
    var d: Int = { def f: Int = d; { def g: Int = d; 3 } }
    var e: Int = { val t: Empty = new Empty; def f: Int = e; 3 }
    var g: Int = { object Inner { def f: Int = g }; 3 }
    var h: Int = { class Reads { def f: Int = h }; val t: Reads = new Reads; 3 }
    var i: Int = { def f: Int = i; Constants.three }
    var j: Int = { def f: Int = j; if true then 3 else 7 }
    var l: Int = { val t: (Int, Int) = (1, 2); def f: Int = l + t._1; 3 }
    var m: Int = { val t: Option[Int] = Some(1); def f: Int = m; 3 }
    var o: Int = { def f: Int = o; if Constants.yes then 3 else 7 }
    var p: Int = { def f: Int = p; Constants.threeOf(k) }
    var q: Int = Constants.bodyOf(q)
    var r: Int = { val t: Int = Constants.plainThree; def f: Int = r; 3 }
    var s: Int = { val t: Int = if true then k else 2; def f: Int = s; 3 }
    a + b + c + d + e + g + h + i + j + l + m + o + p + q + r + s - 45
  // A level that is itself such a block is no level of scalac's tree.
  def foldedLevel(n: Int): Int =
    def first: Int = later
    val held: Int = 5
    def later: Int = 1
    3
  def foldedUnitValue(n: Int): Int =
    val u: Unit = { def f: Unit = u; () }
    3
  // A type test's type names nothing in scalac's tree: a member class of a later object in a typed pattern, in a
  // type test and in a cast (the test of its enclosing instance is made after the check).
  def memberTypes(n: Int): Int =
    def typed(x: Any): Int = x match { case l: Later.Inner => 1; case _ => 0 }
    def wild(x: Any): Int = x match { case _: Later.Mid.Deep => 1; case _ => 0 }
    def tested(x: Any): Int = if x.isInstanceOf[Later.Inner] then 1 else 0
    def cast(x: Any): Int = if n < 0 then x.asInstanceOf[Later.Inner].hashCode else 0
    def caught(x: Int): Int = try x catch { case e: Later.Failure => 0 }
    val held: Int = typed(n) + wild(n) + tested(n) + cast(n) + caught(n)
    object Later { class Inner; object Mid { class Deep }; class Failure extends RuntimeException }
    held
  // A constructor pattern selects a member class's companion through the path it writes: through a parameter of
  // the later object's type, and by the bare name inside the object itself, the object is not named.
  def memberPatternPaths(n: Int): Int =
    def pick(l: Later.type, x: Any): Int = x match { case l.Inner(v) => v; case _ => 0 }
    def deep(m: Later.Mid.type, x: Any): Int = x match { case m.Deep(v) => v; case _ => 0 }
    val held: Int = n
    object Later:
      case class Inner(v: Int) { def same(x: Any): Int = x match { case Inner(w) => w + v; case _ => 0 } }
      object Mid { case class Deep(v: Int) }
      def own(x: Any): Int = x match { case Inner(v) => v; case _ => 0 }
    held + pick(Later, n) + deep(Later.Mid, n) + Later.own(n)
  // What a local trait's member reads is named where the member stands: what its calls, its mixers and its
  // initialiser pass for it is not in scalac's tree when the check runs.
  def traitMemberLater(n: Int): Int =
    def use(t: Source): Int = t.get
    val held: Int = n
    trait Source { def get: Int = late }
    lazy val late: Int = n
    held + use(new Source {}) - n
  def traitMixerBefore(n: Int): Int =
    class Mixer extends Source { override def get: Int = super.get }
    val held: Int = n
    trait Source { val got: Int = late; lazy val slow: Int = late; def get: Int = late }
    lazy val late: Int = n
    val m = new Mixer()
    held + m.get + m.got + m.slow - 3 * n
  // In an inline method's expansion a creation selected for a parameter is its argument.
  def projected(n: Int): Int =
    def first: Int = make(n)
    def second: Int = wrap(n)
    val got: Int = first + second
    class Mid(val v: Int)
    case class Wrapped(v: Int)
    inline def make(x: Int): Int = new Mid(x).v
    inline def wrap(x: Int): Int = Wrapped(x).v
    got - n
  // An alias that is more than its class applied to its own parameters stays the creation's type.
  def aliasPartial(n: Int): Int =
    type Short[A] = Later[A, Int]
    def first: Int = new Short[String](n).v
    val held: Int = first
    class Later[A, B](val v: Int)
    held
  def aliasSwapped(n: Int): Int =
    type Short[A, B] = Later[B, A]
    def first: Int = new Short[Int, String](n).v
    val held: Int = first
    class Later[A, B](val v: Int)
    held
  def main(args: Array[String]): Unit =
    println(List(inlineOver(3), lazyBetween(3), lazyAhead(3), noValue(3), new Members(3).first, typeOnly(3), traitParent(3), typeArgument(3), folded(3), recursion(3), wildcard(3)).mkString(" "))
    println(List(aliasEarlier(3), aliasAnonymous(3), aliasOfAlias(3), foldedAnd(3), foldedOr(3), companionAfter(3)).mkString(" "))
    println(List(aliasAfterComment(3), aliasInParentheses(3), aliasInBackquotes(3), aliasAnonymousAfterComment(3), aliasOfAnObject(3), foldedBlocks(3)).mkString(" "))
    println(List(foldedByPurity(3), foldedLevel(3), foldedUnitValue(3), aliasPartial(3), aliasSwapped(3), memberTypes(3)).mkString(" "))
    println(List(traitMemberLater(3), traitMixerBefore(3), projected(3), memberPatternPaths(3)).mkString(" "))
