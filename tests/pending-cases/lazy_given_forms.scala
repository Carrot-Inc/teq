//> using platform jvm
// (JavaScript and the interpreter left out.)
// Pending on uncache-given-aliases (queued): dotty's UncacheGivenAliases
// ported whole (`needsCache` over the right-hand side's type, `Types.isStable`
// with its recursive branches); the expectation is scalac 3.8.4's.
// UncacheGivenAliases.needsCache by the right-hand side's type (UncacheGivenAliases.scala 31-37): a given alias whose
// right-hand side's type is a stable TermRef of no prefix, a stable TermRef through its enclosing class's `this`, or a
// ThisType is a def, evaluated at each use; any other is a lazy val, evaluated once. Each alias prints when its
// right-hand side runs and is read twice: a block's type is its result's, a branch's the lub of the branches' (a
// throwing one dropped), an ascription's the type written; a var, a def, a by-name parameter, a member of an
// enclosing class, a top-level val, a literal are cached; a val, a lazy val, a parameter, an object, `this`, an outer
// `this`, a `this.type` method are not; a local alias of a local alias made a def is cached, a member's of a member's
// not (scalac 3.8.4's denotations).
val topVal = "topVal"
var topVar = "topVar"
object TopObj
given t1: CharSequence = { println("t1 top val"); topVal }
given t2: CharSequence = { println("t2 top var"); topVar }
given t3: Any = { println("t3 top object"); TopObj }
given t4: Any = { println("t4 other file"); OtherFile.member }
object TopMain:
  def run(): Unit =
    for _ <- 1 to 2 do { t1; t2; t3; t4 }

object OtherFile:
  val member = "member"

trait Tr:
  val inherited: String = "inherited"
class Outer(val p: String):
  val outerVal = "outerVal"
  class Inner extends Tr:
    val v = "v"
    lazy val lz = "lz"
    var mv = "mv"
    def d = "d"
    def self: this.type = this
    def selfApp(): this.type = this
    object Obj { override def toString = "Obj" }
    given g1: String = { println("g1 block"); v }
    given g2: CharSequence = if p.nonEmpty then { println("g2 if"); v } else v
    given g21: CharSequence = if p.nonEmpty then { println("g21 if two"); v } else lz
    given g22: Any = { println("g22 given"); g7 }
    given g23: Any = { println("g23 if this"); if p.nonEmpty then this else this }
    given g3: CharSequence = if p.isEmpty then throw new Exception else { println("g3 if-throw"); v }
    given g4: CharSequence = { println("g4 ascribed"); (v: CharSequence) }
    given g5: CharSequence = { println("g5 var"); mv }
    given g6: CharSequence = { println("g6 def"); d }
    given g7: CharSequence = { println("g7 lazy"); lz }
    given g8: Any = { println("g8 self"); self }
    given g9: Any = { println("g9 selfApp"); selfApp() }
    given g10: Any = { println("g10 obj"); Obj }
    given g11: CharSequence = { println("g11 outer member"); outerVal }
    given g12: Any = { println("g12 outer this"); Outer.this }
    given g13: CharSequence = { println("g13 inherited"); inherited }
    given g14: CharSequence = { println("g14 match"); p match { case _ => v } }
    given g15: CharSequence = { println("g15 try"); try v catch { case _: Exception => v } }
    given g16: CharSequence = { println("g16 outer param"); p }
    given g17: Int = { println("g17 literal"); 1 }
    given g18: CharSequence = { println("g18 local"); val w = v; w }
    given g19: CharSequence = { println("g19 this.v"); this.v }
    given g20: Any = { println("g20 this"); this }
    def all(): Unit =
      for _ <- 1 to 2 do
        g1; g2; g21; g22; g23; g3; g4; g5; g6; g7; g8; g9; g10; g11; g12; g13; g14; g15; g16; g17; g18; g19; g20
  def local(x: String, y: => String): Unit =
    var z = "z"
    val w = "w"
    lazy val lw = "lw"
    given l1: String = { println("l1 param"); x }
    given l2: CharSequence = { println("l2 byname"); y }
    given l3: CharSequence = { println("l3 var"); z }
    given l4: CharSequence = { println("l4 val"); w }
    given l5: CharSequence = { println("l5 lazy"); lw }
    given l6: CharSequence = { println("l6 this member"); outerVal }
    given l7: Any = { println("l7 this"); this }
    object LO
    given l8: Any = { println("l8 local object"); LO }
    def ld: String = "ld"
    given l9: CharSequence = { println("l9 local def"); ld }
    given l10: Any = { println("l10 local given"); l4 }
    for _ <- 1 to 2 do
      l1; l2; l3; l4; l5; l6; l7; l8; l9; l10
trait Tg:
  val tv = "tv"
  var tm = "tm"
  given tg1: CharSequence = { println("tg1 trait val"); tv }
  given tg2: CharSequence = { println("tg2 trait var"); tm }
  given tg3: Any = { println("tg3 trait this"); this }
class Tc extends Tg
class Ctor(val q: String, r: String):
  given c1: CharSequence = { println("c1 val param"); q }
  given c2: CharSequence = { println("c2 plain param"); r }
object Main:
  def main(args: Array[String]): Unit =
    TopMain.run()
    val o = new Outer("p")
    new o.Inner().all()
    o.local("x", "y")
    val c = new Ctor("q", "r")
    for _ <- 1 to 2 do { c.c1; c.c2 }
    val tc = new Tc
    for _ <- 1 to 2 do { tc.tg1; tc.tg2; tc.tg3 }
