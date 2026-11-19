def emptyTop(using ev: Numeric[String]): Int = 1
class Cl { def m(using ev: Numeric[String]): Int = 1 }
object Ob { def m(using ev: Numeric[String]): Int = 1 }
trait Tr { def m(using ev: Numeric[String]): Int = 1 }
class Ctor(using ev: Numeric[String])
object Ext:
  extension (i: Int) def ext(using ev: Numeric[String]): Int = i
case class Box(i: Int)
object Box:
  def apply(i: String)(using ev: Numeric[String]): Box = Box(1)
given gi(using ev: Numeric[String]): Int = 1
trait Svc { def m(using ev: Numeric[String]): Int }
given gimpl: Svc with { def m(using ev: Numeric[String]): Int = 1 }
def bound[A: Numeric]: Int = 1
object Uses:
  val a = pkg.top
  val b = pkg.po.inPo
  val c = emptyTop
  val d = Cl().m
  val e = Ob.m
  def f(t: Tr) = t.m
  val g = new Ctor
  val h = Ext.ext(1)
  val i = Box("x")
  val j = new { def anon(using ev: Numeric[String]): Int = 1; val k = anon }
  def l = { class Local { def m(using ev: Numeric[String]): Int = 1 }; Local().m }
  def n = { def inner(using ev: Numeric[String]): Int = 1; inner }
  val o = summon[Int](using gi)
  val p = gimpl.m
  val q = bound[String]
// A missing given names its callee as scalac locates it (PlainPrinter.locatedText, locationText with effectiveOwner
// and isEmptyPrefix): one line for each kind of owner; scalac 3.8.4 gives each line whole at one column less.
// expect: b.scala:16:18: error: No given instance of type Numeric[String] was found for parameter ev of method top in package pkg
// expect: b.scala:17:22: error: No given instance of type Numeric[String] was found for parameter ev of method inPo in package pkg.po
// expect: b.scala:18:19: error: No given instance of type Numeric[String] was found for parameter ev of method emptyTop
// expect: b.scala:19:17: error: No given instance of type Numeric[String] was found for parameter ev of method m in class Cl
// expect: b.scala:20:15: error: No given instance of type Numeric[String] was found for parameter ev of method m in object Ob
// expect: b.scala:21:21: error: No given instance of type Numeric[String] was found for parameter ev of method m in trait Tr
// expect: b.scala:22:19: error: No given instance of type Numeric[String] was found for parameter ev of constructor Ctor in class Ctor
// expect: b.scala:23:21: error: No given instance of type Numeric[String] was found for parameter ev of method ext in object Ext
// expect: b.scala:24:19: error: No given instance of type Numeric[String] was found for parameter ev of method apply in object Box
// expect: b.scala:25:75: error: No given instance of type Numeric[String] was found for parameter ev of method anon
// expect: b.scala:26:81: error: No given instance of type Numeric[String] was found for parameter ev of method m in class Local
// expect: b.scala:27:65: error: No given instance of type Numeric[String] was found for parameter ev of method inner
// expect: b.scala:28:31: error: No given instance of type Numeric[String] was found for parameter ev of given instance gi
// expect: b.scala:29:18: error: No given instance of type Numeric[String] was found for parameter ev of method m in object gimpl
// expect: b.scala:30:24: error: No given instance of type Numeric[String] was found for a context parameter of method bound
// expect: 15 errors found
