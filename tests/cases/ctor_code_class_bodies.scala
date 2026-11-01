// The constructor's code reading the class's parameter accessor by its identifier (dotty's `Constructors.intoConstr`):
// a lambda, a lambda of a SAM type (a Java interface's, an abstract class's, one inside another), a local function and
// the default of a local function's parameter read the constructor's parameter (what the superclass got); the walk
// stops at every class's body (`noDirectRefsFrom`), so a partial function literal's cases, an explicit anonymous class
// (inside a lambda too), a lambda inside one and a local class read the accessor, the subclass's field.
abstract class Fn:
  def apply(): Int

class A(val s: Int):
  val lam = () => s
  val sam: java.util.function.IntSupplier = () => s
  val absSam: Fn = () => s
  val nestedSam: Fn = () => { val g: java.util.function.IntSupplier = () => { val h: Fn = () => s; h() }; g.getAsInt() }
  val anonInLam = () => new Fn { def apply() = s }
  val pf: PartialFunction[Int, Int] = { case _ => s }
  val anon: Fn = new Fn { def apply() = s }
  val lamInAnon: Fn = new Fn { val k = () => s; def apply() = k() }
  val local = { class L { def g = s }; L().g }
  val localFun = { def h(n: Int) = s + n; h(10) }
  val localDefault = { def f = g(); def g(k: Int = s) = k; f }
class B(override val s: Int) extends A(s + 1)

@main def run(): Unit =
  val b = B(2)
  println(s"${b.lam()} ${b.sam.getAsInt()} ${b.absSam()} ${b.nestedSam()} ${b.anonInLam()()} ${b.localFun} ${b.localDefault}")
  println(s"${b.pf(0)} ${b.anon()} ${b.lamInAnon()} ${b.local}")
  val a = A(5)
  println(s"${a.lam()} ${a.sam.getAsInt()} ${a.absSam()} ${a.nestedSam()} ${a.anonInLam()()} ${a.pf(0)} ${a.anon()} ${a.lamInAnon()} ${a.local} ${a.localFun} ${a.localDefault}")
