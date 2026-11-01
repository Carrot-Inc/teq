// An inline call in constructor code whose expansion reads the class's parameter accessor by its identifier (dotty's
// `Constructors.intoConstr` over the expanded code): a closure, a lambda of a SAM type (a Java interface's or an abstract
// class's, whose body `ExpandSAMs` keeps a local function), a local function, a nested expansion and a local lazy val
// read the constructor's parameter (what the superclass got); a partial function literal (`toPartialFunction` makes it a
// class), an explicit anonymous class, a local class and a selection `this.s` read the accessor, the subclass's field.
abstract class Fn:
  def apply(): Int

class A(val s: Int):
  inline def pf: PartialFunction[Int, Int] = { case _ => s }
  inline def sam: java.util.function.IntSupplier = () => s
  inline def absSam: Fn = () => s
  inline def clos = () => s
  inline def loc = { def f = s; f }
  inline def nested = loc
  inline def withLazy = { lazy val t = s; t }
  inline def anon: Fn = new Fn { def apply() = s }
  inline def local = { class L { def g = s }; L().g }
  inline def samThis: Fn = () => this.s
  inline def closThis = () => this.s
  val a = pf
  val b = sam
  val c = absSam
  val d = clos
  val e = loc
  val f = nested
  val g = withLazy
  val h = anon
  val i = local
  val j = samThis
  val k = closThis
class B(override val s: Int) extends A(s + 1)

@main def run(): Unit =
  val b = B(2)
  println(s"${b.a(0)} ${b.b.getAsInt()} ${b.c()} ${b.d()} ${b.e} ${b.f} ${b.g} ${b.h()} ${b.i} ${b.j()} ${b.k()}")
  val a = A(5)
  println(s"${a.a(0)} ${a.b.getAsInt()} ${a.c()} ${a.d()} ${a.e} ${a.f} ${a.g} ${a.h()} ${a.i} ${a.j()} ${a.k()}")
