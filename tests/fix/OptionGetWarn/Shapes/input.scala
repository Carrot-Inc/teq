/*
rule = OptionGetWarn
*/
package fix

type Opt[A] = Option[A]
class Ann extends scala.annotation.StaticAnnotation

val top: Option[Int] = None
def topDef: Option[Int] = None

// A `get` of the program's own.
trait Holder[A]:
  def get: A

enum E:
  case A(o: Option[Int])
  def viaCase: Int = this match
    case A(o) => o.get // assert: OptionGetWarn

class Base:
  val inherited: Option[Int] = None

class Shapes(val cp: Option[Int], cq: Option[Int], var cv: Option[Int]) extends Base:
  val declared: Option[Int] = None
  val inferredOption = Option(1)
  val inferredSome = Some(1)
  val inferredNone = None
  val inferredIf = if cp.isEmpty then Some(1) else None
  val alias: Opt[Int] = None
  val annotated: Option[Int] @Ann = None
  val union: Option[Int] | Null = None
  lazy val lz: Option[Int] = None
  var v: Option[Int] = None
  def nullary: Option[Int] = None
  def nullaryInferred = Option(1)
  def empty(): Option[Int] = None
  def poly[A]: Option[A] = None
  def polyArgs[A](a: A): Option[A] = Some(a)
  def curried(a: Int)(b: Int): Option[Int] = None
  def fn: Int => Option[Int] = _ => None
  val fnVal: Int => Option[Int] = _ => None
  given g: Option[Int] = None
  val self: Shapes = this

  // The receiver a parameter: by name, by value, of a bounded type parameter, a Some, a using clause's, an extension's.
  def byName(o: => Option[Int]): Int = o.get // assert: OptionGetWarn
  def param(o: Option[Int]): Int = o.get // assert: OptionGetWarn
  def typeParam[T <: Option[Int]](t: T): Int = t.get // assert: OptionGetWarn
  def some(s: Some[Int]): Int = s.get
  def usingParam(using o: Option[Int]): Int = o.get // assert: OptionGetWarn
  extension (o: Option[Int]) def ext: Int = o.get // assert: OptionGetWarn

  def uses(other: Shapes, xs: List[Option[Int]], holder: Holder[Int]): Unit =
    // The receiver a member: of this class, of another instance, inherited, of each declared or inferred type.
    declared.get // assert: OptionGetWarn
    this.declared.get // assert: OptionGetWarn
    other.declared.get // assert: OptionGetWarn
    self.self.declared.get // assert: OptionGetWarn
    inherited.get // assert: OptionGetWarn
    inferredOption.get // assert: OptionGetWarn
    inferredSome.get
    inferredNone.get // assert: OptionGetWarn
    inferredIf.get // assert: OptionGetWarn
    alias.get // assert: OptionGetWarn
    annotated.get // assert: OptionGetWarn
    union.get // assert: OptionGetWarn
    lz.get // assert: OptionGetWarn
    v.get // assert: OptionGetWarn
    cp.get // assert: OptionGetWarn
    cq.get // assert: OptionGetWarn
    cv.get // assert: OptionGetWarn
    // The receiver a method's result, an applied function value's, a given.
    nullary.get // assert: OptionGetWarn
    nullaryInferred.get // assert: OptionGetWarn
    empty().get // assert: OptionGetWarn
    poly[Int].get // assert: OptionGetWarn
    poly.get // assert: OptionGetWarn
    polyArgs(1).get // assert: OptionGetWarn
    polyArgs[Int](1).get // assert: OptionGetWarn
    curried(1)(2).get // assert: OptionGetWarn
    fn(1).get // assert: OptionGetWarn
    fnVal(1).get // assert: OptionGetWarn
    fn.apply(1).get // assert: OptionGetWarn
    g.get // assert: OptionGetWarn
    summon[Option[Int]].get // assert: OptionGetWarn
    // The receiver in parentheses, ascribed, annotated; a top-level definition, a companion's member, a new instance's.
    (declared).get // assert: OptionGetWarn
    (declared: Option[Int]).get // assert: OptionGetWarn
    (declared: @unchecked).get // assert: OptionGetWarn
    top.get // assert: OptionGetWarn
    topDef.get // assert: OptionGetWarn
    fix.top.get // assert: OptionGetWarn
    Shapes.companionVal.get // assert: OptionGetWarn
    new Shapes(None, None, None).declared.get // assert: OptionGetWarn
    // The receiver an element, a placeholder, a lambda's parameter, a generator's.
    xs.head.get // assert: OptionGetWarn
    xs(0).get // assert: OptionGetWarn
    xs.map(_.get) // assert: OptionGetWarn
    xs.map(o => o.get) // assert: OptionGetWarn
    xs.map { case o => o.get } // assert: OptionGetWarn
    xs.foreach { (o: Option[Int]) => o.get } // assert: OptionGetWarn
    for o <- xs yield o.get // assert: OptionGetWarn
    for o <- xs do o.get // assert: OptionGetWarn
    // The receiver a local: declared, inferred, bound by a pattern.
    val local: Option[Int] = None
    local.get // assert: OptionGetWarn
    val localInferred = xs.head
    localInferred.get // assert: OptionGetWarn
    val (a, b) = (Option(1), 2)
    a.get // assert: OptionGetWarn
    val first :: _ = xs: @unchecked
    first.get // assert: OptionGetWarn
    def localDef: Option[Int] = None
    localDef.get // assert: OptionGetWarn
    lazy val localLazy = Option(1)
    localLazy.get // assert: OptionGetWarn
    var localVar = Option(1)
    localVar.get // assert: OptionGetWarn
    // The receiver a library's call: Option's and Some's constructors, None, Option's own methods.
    Option(1).get // assert: OptionGetWarn
    Some(1).get
    None.get // assert: OptionGetWarn
    Option.apply(1).get // assert: OptionGetWarn
    Option.empty[Int].get // assert: OptionGetWarn
    declared.map(_ + 1).get // assert: OptionGetWarn
    declared.orElse(None).get // assert: OptionGetWarn
    locally(declared).get // assert: OptionGetWarn
    // The receiver a case's binder, a block, a conditional.
    cp match
      case o @ Some(_) => o.get
      case o => o.get // assert: OptionGetWarn
    cp match
      case o: Option[Int] => o.get // assert: OptionGetWarn
    { declared }.get // assert: OptionGetWarn
    (if true then declared else declared).get // assert: OptionGetWarn
    // The selection itself: selected from, backquoted, ascribed, a function's body.
    declared.get.toString // assert: OptionGetWarn
    declared.`get` // assert: OptionGetWarn
    declared.get: Int // assert: OptionGetWarn
    val f: Option[Int] => Int = _.get // assert: OptionGetWarn
    val h = (o: Option[Int]) => o.get // assert: OptionGetWarn
    // A map's value and its lookup by a key; other members named `get`, the program's own and a Try's; a chain with none.
    val m = Map(1 -> Option(1))
    m(1).get // assert: OptionGetWarn
    m.get(1)
    holder.get
    scala.util.Try(1).get
    xs.flatMap(_.map(_ + 1)).head
    ()

object Shapes:
  val companionVal: Option[Int] = None
