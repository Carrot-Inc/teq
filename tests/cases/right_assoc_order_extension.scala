// A right-associative operator's left operand is evaluated before its right one wherever `Typer.typedInfixOp`
// lifts it (the impure argument of `r.op(l)` bound ahead): an extension method `+::`, which takes the left operand in its receiver's clause, in plain code. One statement a kind of left
// operand (k01 a given of an object ... k36 a block beside a top-level given), `Log` recording the order: `lr`, or
// `r` where the left operand has no effect. Master read a top-level lazy value, a top-level given, an object with an
// initializer, an inline parameter and a local lazy value on the right before the left operand.
import scala.compiletime.summonInline
object Log:
  val on: Boolean = true
  private var s: String = ""
  def apply(x: String): Unit = { s = s + x }
  def take(): String = { val r = s; s = ""; r }
class Elem(val n: Int) { override def toString: String = "E" + n }
class GElem(n: Int) extends Elem(n)
class SElem(n: Int) extends Elem(n)
object SElem:
  given sv: SElem = { Log("l"); new SElem(1) }
  given svl: List[SElem] = { Log("l"); List(new SElem(1)) }
class TElem(n: Int) extends Elem(n)
class UElem(n: Int) extends Elem(n)
class WElem(n: Int) extends Elem(n)
class HElem(n: Int) extends Elem(n)
class XElem(n: Int) extends Elem(n)
class YElem(n: Int) extends Elem(n)
given tg: TElem = { Log("l"); new TElem(1) }
given tgl: List[TElem] = { Log("l"); List(new TElem(1)) }
given tu: UElem = { Log("l"); new UElem(1) }
given tul: List[UElem] = { Log("l"); List(new UElem(1)) }
given tw: WElem = { Log("l"); new WElem(1) }
given twl: List[WElem] = { Log("l"); List(new WElem(1)) }
given th: HElem = { Log("l"); new HElem(1) }
given thl: List[HElem] = { Log("l"); List(new HElem(1)) }
given tx: XElem = { Log("l"); new XElem(1) }
given txl: List[XElem] = { Log("l"); List(new XElem(1)) }
given ty: YElem = { Log("l"); new YElem(1) }
given tyl: List[YElem] = { Log("l"); List(new YElem(1)) }
class ZElem(n: Int) extends Elem(n)
given tz: ZElem = { Log("l"); new ZElem(1) }
given tzl: List[ZElem] = { Log("l"); List(new ZElem(1)) }
class Wrap(val v: Any)
class Loud extends Elem(1) { Log("l") }
case class Cs(n: Int) { Log("l") }
lazy val tlz: Elem = { Log("l"); new Elem(1) }
lazy val tlzl: List[Any] = { Log("l"); List[Any](new Elem(1)) }
class Ctx
object Ctx:
  given ctx: Ctx = new Ctx
class Box(val es: List[Any]):
  def +:(e: Any): Box = new Box(e :: es)
  def ++:(e: Any)(k: Int): Box = new Box(e :: k :: es)
  def +++:(e: Any)(using c: Ctx): Box = new Box(e :: es)
  override def toString: String = es.mkString("Box(", ",", ")")
class Bag(val es: List[Any]):
  override def toString: String = es.mkString("Bag(", ",", ")")
extension (e: Any) def +::(b: Bag): Bag = new Bag(e :: b.es)
def line(tag: String, r: Any): Unit = println(tag + " " + Log.take() + " " + r)
lazy val trz: Bag = { Log("r"); new Bag(Nil) }
given trg: Bag = { Log("r"); new Bag(Nil) }
object D:
  given gv: GElem = { Log("l"); new GElem(1) }
  given gw: GElem = { Log("l"); new GElem(1) }
  given gx: GElem = { Log("l"); new GElem(1) }
  inline def ir(): Bag = { Log("r"); new Bag(Nil) }
  object Mod { Log("l"); override def toString: String = "Mod" }
  def mkw(): Wrap = { Log("l"); new Wrap(new Elem(1)) }
  def pf[T]: Elem = { Log("l"); new Elem(1) }
  def da(n: Int): Elem = { Log("l"); new Elem(1) }
  inline def useI(inline p: Elem): Any = p +:: rd()
  def useL(): Any = { lazy val ll: Elem = { Log("l"); new Elem(1) }; ll +:: rd() }
  inline def il: Elem = { Log("l"); new Elem(1) }
  transparent inline def tl: Elem = { Log("l"); new Elem(1) }
  lazy val lz: Elem = { Log("l"); new Elem(1) }
  def df: Elem = { Log("l"); new Elem(1) }
  var vr: Elem = new Elem(1)
  def rd(): Bag = { Log("r"); new Bag(Nil) }
  def rv(): Bag = { vr = new Elem(9); Log("r"); new Bag(Nil) }
  lazy val rz: Bag = { Log("r"); new Bag(Nil) }
  lazy val ry: Bag = { Log("r"); new Bag(Nil) }
  lazy val rx: Bag = { Log("r"); new Bag(Nil) }
  def useP(p: Elem): Any = p +:: rd()
  def useB(p: => Elem): Any = p +:: rd()
  def run(): Unit =
    line("k01_given", gv +:: rd())
    line("k03_inline_call", il +:: rd())
    line("k04_transparent_inline", tl +:: rd())
    line("k05_lazy_val", lz +:: rd())
    line("k06_def", df +:: rd())
    line("k07_var", vr +:: rv())
    line("k08_literal", 7 +:: rd())
    line("k09_parameter", useP(new Elem(1)))
    line("k10_by_name_parameter", useB({ Log("l"); new Elem(1) }))
    line("k11_block", { Log("l"); new Elem(1) } +:: rd())
    line("k12_block_right_lazy_val", { Log("l"); new Elem(1) } +:: rz)
    line("k13_given_right_lazy_val", gw +:: ry)
    line("k14_def_right_lazy_val", df +:: rx)
    line("k15_toplevel_given", tg +:: rd())
    line("k17_given_cast", gx.asInstanceOf[Elem] +:: rd())
    line("k19_toplevel_lazy_val", tlz +:: rd())
    line("k20_block_right_inline_call", { Log("l"); new Elem(1) } +:: ir())
    line("k21_toplevel_given_right_inline_call", th +:: ir())
    line("k24_inline_parameter", useI({ Log("l"); new Elem(1) }))
    line("k25_select_on_a_call", mkw().v +:: rd())
    line("k26_new_with_an_impure_argument", new Elem({ Log("l"); 1 }) +:: rd())
    line("k27_new_of_a_class_with_an_initializer", new Loud +:: rd())
    line("k28_polymorphic_def", pf[Int] +:: rd())
    line("k29_def_with_an_argument", da(1) +:: rd())
    line("k30_case_class_with_an_initializer", Cs(1) +:: rd())
    line("k31_ascription_over_a_toplevel_given", (tz: Elem) +:: rd())
    line("k32_if_expression", (if Log.on then { Log("l"); new Elem(1) } else new Elem(1)) +:: rd())
    line("k33_object_with_an_initializer", Mod +:: rd())
    line("k34_local_lazy_val", useL())
    line("k35_block_right_toplevel_lazy_val", { Log("l"); new Elem(1) } +:: trz)
    line("k36_block_right_toplevel_given", { Log("l"); new Elem(1) } +:: trg)
@main def main(): Unit = D.run()
