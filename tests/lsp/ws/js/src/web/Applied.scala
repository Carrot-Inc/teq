package web

import scala.language.implicitConversions

// Function values applied (`f(x)`, the `apply` the typer inserts on a function type, a partial
// function, a SAM-typed value or a tuple): each written name is recorded as its unapplied use is,
// a function value's as a reference and a method's whose result is applied as a call. The driver
// copies this file and AppliedMacro.scala into the JVM project and the JavaScript project on
// scala-library.
object Applied:
  val field: Int => Int = x => x + 1
  val zero: () => Int = () => 3
  val partial: PartialFunction[Int, Int] = { case x => x + 1 }
  trait Sam:
    def run(x: Int): Int
  val sam: Sam = x => x + 1
  trait Ap:
    def apply(x: Int): Int
  val ap: Ap = x => x + 1
  trait Defaulted:
    def apply(x: Int, y: Int = 2): Int
  def mk(): Int => Int = x => x + 1
  def method(x: Int): Int = x + 1
  def many(xs: Int*): Int = xs.size
  extension (x: Int)
    def fun: Int => Int = y => x + y
  trait Has:
    val member: Int => Int
  given inst: Has with
    val member: Int => Int = x => x
  trait Ctx:
    def f: Int => Int
  def implicitUse(using ctx: Ctx): Int = ctx.f(2)
  def defaults(x: Int = field(10)): Int = x
  def byname(bn: => Int => Int): Int = bn(11)
  def guard(f: Int => Boolean, x: Int): Int = x match
    case y if f(y) => y
    case _ => 0
  class Holder(val hf: Int => Int):
    def viaThis(x: Int): Int = this.hf(x) + hf(x)
  def run(g: Int => Int, holder: Holder, dd: Defaulted, f3: (Int, Int, Int) => Int): Int =
    val h = g
    val n = h
    val a = h(1)
    val b = g(2)
    val explicit = h.apply(3)
    val inf = h apply 4
    val z = zero()
    val p = partial(5)
    val s = sam.run(6)
    val aa = ap(7)
    val t = this.field(8)
    val u = field(9)
    val q = Applied.field(10)
    val made = mk()(11)
    val fs = (h, g)
    val tup = fs(0)(12)
    val eta = method
    val et = eta(13)
    val ext = 1.fun(14)
    val gi = inst.member(15)
    val cur: Int => Int => Int = x => y => x + y
    val cu = cur(1)(2)
    val two: (Int, Int) => Int = (x, y) => x + y
    val tw = two(1, 2)
    val three = f3(1, 2, 3)
    val poly = [A] => (x: A) => x
    val po = poly[Int](16)
    val repeated: Seq[Int] => Int = xs => xs.size
    val va = repeated(Seq(1, 2))
    val vari: Seq[Int] => Int = many
    val vr = vari(Seq(3, 4))
    val mf = AppliedMacro.make
    val mr = mf(20)
    val hd = holder.hf(21) + dd(22)
    var total = a + b + z + p + s + aa + t + u + q + made + tup + et + ext + gi + cu + tw + three + po + va + vr + mr + hd
    once:
      total += h(23)
    total + explicit + inf + n(24)
  def once(c: => Unit): Unit = c
  type Task = Int
  type RunAction = Task => String
  def useRun: RunAction = t => t.toString
  // The two lines of the report: a local val of a function alias applied in a colon block, and a
  // parameter of a generic function type applied in a pattern definition.
  def entityList[CAT](current: CAT, mkArgs: CAT => (Int, String)): String =
    val runAction = useRun
    once:
      runAction(1)
    val (_count, label) = mkArgs(current)
    label
  class Up extends (Int => Int):
    def apply(x: Int): Int = x
    def update(x: Int, y: Int): Unit = ()
  def updateTest(f: Up): Unit = f(1) = 2
  def strings(h: Int => Int): String = s"value=${h(17)}"
  object same:
    def apply(x: Int): Int = x
  def shadow(same: Int => Int): Int = same(18)
  def objectCall: Int = same(19)
  // A member written after a qualifier of its own name is the member, and an inline method
  // whose result is applied is called.
  object Factories:
    val factory: Int => Int => Int = x => y => x + y
  inline def factory(): Int => Int = Factories.factory(1)
  def collisions(f: Int => Int): Int =
    val apply: Int => Int = x => x + 1
    val head = (f, f)
    val _1 = (f, f)
    apply.apply(30) + head.head(31) + _1(0)(32) + factory()(33)
  def tupleNamedApply(f: Int => Int): Int =
    val apply = (f, f)
    apply.apply(0)(34)
  // Imports' renames, nested tuples, a qualifier behind an ascription or in a block.
  object Tables:
    val pair: (Int => Int, Int => Int) = (x => x + 1, x => x + 2)
    val _1: (Int => Int, Int => Int) = (x => x, x => x)
  def renamedAndNested(f: Int => Int): Int =
    import Tables.{pair as twin, _1 as first}
    val nested = ((f, f), (f, f))
    val _2 = ((f, f), (f, f))
    val apply: Int => Int = x => x + 2
    val head = (f, (x: Int) => x)
    twin(0)(40) + first(1)(41) + nested(0)(1)(42) + _2(1)(0)(43) + (apply: Int => Int).apply(44) + { head }.head(45)
  // Imports renaming other values to a written member's name, and members of a written
  // tuple's element.
  object Elsewhere:
    val pair: (Int => Int, Int => Int) = (x => x, x => x)
    val g: Int => Int = x => x
  def unrelatedRenames(f: Int => Int): Int =
    import Elsewhere.{pair as head, g as apply}
    val g: Int => Int = x => x + 1
    val pair = (f, f)
    val tuples = ((f, f), (f, f), (f, f))
    pair.head(50) + g.apply(51) + tuples._1(0)(52) + tuples._2(1)(53) + head(0)(54) + apply(55)
  // Values a conversion makes functions of, tuples of SAM and partial function values, an inline
  // `apply` of an object, members spelled as their qualifiers.
  case class Config(name: String)
  object Config:
    given config: Conversion[Config, Int => Int] = c => x => x + c.name.length
  case class Spec(n: Int)
  object Spec:
    implicit def spec(s: Spec): Int => Int = x => x + s.n
  object Registry:
    val handlers: Map[String, Int => Int] = Map("a" -> (x => x + 1))
  object handlers:
    inline def apply(name: String): Int => Int = Registry.handlers(name)
  class Handler(val handler: Int => Int)
  class Maker:
    def maker(): Int => Int = x => x
  object SamTables:
    val _1: (Ap, Ap) = (x => x, x => x + 1)
  def converted(config: Config, spec: Spec, pf: PartialFunction[Int, Int], ap: Ap, ff: Int => Ap, handler: Handler, maker: Maker): Int =
    import scala.language.implicitConversions
    import SamTables.{_1 as samFirst}
    val pfs = (pf, pf)
    val aps = (ap, ap)
    config(60) + spec(61) + pfs(0)(62) + aps(1)(63) + samFirst(0)(64) + handlers("a")(65) + ff(66)(67) + handler.handler(68) + maker.maker()(69)
  // Conversions spelled like the written name, an `apply` an extension or an
  // implicit class supplies, an implicit def of `using` parameters alone.
  case class Cfg(n: Int)
  object Cfg:
    given cfg: Conversion[Cfg, Int => Int] = c => x => x + c.n
  case class Ob(n: Int)
  object Ob:
    given ob: Conversion[Ob, Ap] = o => x => x + o.n
  case class Ri(n: Int)
  object Ri:
    implicit class Rich(r: Ri):
      def apply(x: Int): Int = x + r.n
  case class Ex(n: Int)
  object Ex:
    extension (e: Ex) def apply(x: Int): Int = x + e.n
  object UseC4:
    import scala.language.implicitConversions
    def spec(): Spec = Spec(2)
    def cfg(): Cfg = Cfg(3)
    def tuples(s: Spec, c: Cfg): Int =
      val spec = (s, s)
      val cfg = (c, c)
      spec(0)(71) + cfg(1)(72)
    def methods(): Int = spec()(73) + cfg()(74)
    def plain(spec: Spec, cfg: Cfg): Int = spec(75) + cfg(76)
  class ConvHolder(val spec: Spec, val cfg: Cfg):
    import scala.language.implicitConversions
    def run(): Int = spec(77) + this.spec(78) + cfg(79) + this.cfg(80)
  object UseC:
    import scala.language.implicitConversions
    val spec: Spec = Spec(1)
    def go(spec: Spec, cfg: Cfg, ob: Ob, ri: Ri, ex: Ex): Int =
      spec(81) + cfg(82) + ob(83) + ri(84) + ex(85) + UseC.spec(86)
    def tup(spec: Spec, cfg: Cfg): Int =
      val specs = (spec, spec)
      val cfgs = (cfg, cfg)
      specs(0)(87) + cfgs(1)(88)
    def explicit(s: Spec, c: Cfg): Int = Spec.spec(s)(89) + Cfg.cfg(c)(90) + Cfg.cfg(c).apply(91)
    implicit def ord(using n: Int): Int => Int = x => x + n
    def usingConv(using Int): Int = ord(92)
    def mkSpec(): Spec = Spec(2)
    val cfgVal: Cfg = Cfg(3)
    def mid(): Int = mkSpec()(93) + cfgVal(94) + UseC.mkSpec()(95)
  // More applied values: a by-name parameter's plain use,
  // an array's and a string's element, a lambda an inline method or a macro expands to applied
  // at once, generators' binders spelled as the start of `for`, a given's own method, an
  // ascribed object's member (the base's member, and the object's that runs), a nullary method
  // beside a conversion of its name, and `apply` written after a function value.
  def plainByName(bv: => Int): Int = bv + bv
  def elements(arr: Array[Int], str: String, grid: Array[Array[Int]]): Int = arr(0) + str(1).toInt + grid(0)(1)
  transparent inline def adder(n: Int): Int => Int = x => x + n
  inline def doubler(): Int => Int = x => x * 2
  def reduced(y: Int): Int = adder(3)(y) + doubler()(y) + AppliedMacro.make(26)
  def generators(fns: List[Int => Int]): List[Int] = for f <- fns; fo <- List(1) yield f(fo)
  trait Counter:
    def count(x: Int): Int
  given counter: Counter with
    def count(x: Int): Int = x
  trait Shape:
    def area(x: Int): Int
  object Square extends Shape:
    def area(x: Int): Int = x * x
  def members: Int = counter.count(5) + (Square: Shape).area(2) + Square.area(3)
  case class Spec2(n: Int)
  object Spec2:
    implicit def spec2(s: Spec2): Int => Int = x => x + s.n
    def spec2(): Spec2 = Spec2(1)
  def nullaryBeside: Int =
    import scala.language.implicitConversions
    Spec2.spec2()(4) + Spec2.spec2(Spec2(2))(5)
  def written(w: Int => Int, curried: Int => Int => Int): Int = w.apply(6) + curried(1).apply(7) + curried.apply(2)(8)
  // A written infix call of a conversion (its argument spelled like it)
  // and the conversion's method value, an extension on an ascribed object's base, a source
  // extension's `apply` applied twice.
  class V(val n: Int)
  object Conv:
    implicit infix def conv(v: V): Int = v.n
  def infixConv(conv: V, other: V): Int = (Conv conv conv) + (Conv conv other)
  val convValue: V => Int = Conv.conv
  trait Plain
  object PlainObj extends Plain:
    def pf(x: Int): Int = x + 100
  extension (p: Plain) def pf(x: Int): Int = x + 1
  def plainAscribed: Int = (PlainObj: Plain).pf(2)
  class Ex3
  // In the companion: an `apply` of this object's own would make `apply(55)` above ambiguous with
  // the import's rename (scalac's E049).
  object Ex3:
    extension (e3: Ex3) def apply(n: Int): Ex3 = e3
  def nestedExtension(e3: Ex3): Ex3 = e3(1)(2)
