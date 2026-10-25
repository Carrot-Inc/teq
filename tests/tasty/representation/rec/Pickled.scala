// The oracle's nonrecursive RECtype (README.md), its second stage, compiled by scalac against the first: `member`'s
// inferred type is the macro's RecursiveType, pickled as a RECtype with no RECthis; `recMember`'s is a RECtype whose
// body names its self, the control.
package repr.rec

class C { type T; val a: Any = null }

// The signed-name pair (the review on b047da41b): two overloads whose first parameter signatures, each a class named
// with the signature separators, flatten alike; `Kinds.signed` calls one of them, its first parameter's signature the
// macro's choice.
class X
class `X, repr.rec.Y`
class `Y, repr.rec.Z`
class Z
object Overloads:
  def f(a: `X, repr.rec.Y`, b: Z): Int = 1
  def f(a: X, b: `Y, repr.rec.Z`): Int = 2

object Pickled:
  val member = Lib.mk
  // An annotation tree's constants by their kinds: check.sh's variant reads `make(false)`, Int leaves for Byte ones.
  val kindsIf = Kinds.make(true)
  // An annotation tree's binding graph: the variant reads `locals(false)`, the inner `x` for the outer one.
  val localRef = Kinds.locals(true)
  // A val whose body scalac's own unpickler cannot read (MatchError(129) on the macro's inlined tree, the review's probe on
  // 334a43adf), its annotation observed apart from the body: the variant reads `@tagged(98)`.
  @tagged(97) val taggedLocals = Kinds.locals(outer = true)
  // A local's annotation inside a body scalac's reflection cannot read (MatchError(129) on forcing it, the reviews'
  // probe on 7a9ec6133 and febb521c4), which no record of the reflection holds: the products' tags observe it (check
  // 11), and the variant reads `@tagged(108)`.
  def blindBody: Int =
    @tagged(107) val v = Kinds.locals(outer = true)
    v
  // The newline pair (the review's probe on 930727870): a string holding a newline before a local whose annotation the
  // products' tags observe, inside a body scalac's reflection cannot read; the variant reads `@tagged(110)`.
  def blindNewline: Int =
    val before = "\n"
    @tagged(109) val v = Kinds.locals(outer = true)
    v
  // The return pair through the fallback (the review's probe on 930727870): a return to one of the two `jump`s inside a
  // body scalac's reflection cannot read; the variant reads `returnTarget(first = false)`.
  def blindReturn: Int =
    val v = Kinds.locals(outer = true)
    val x = Kinds.returnTarget(first = true)
    v
  // The newline/backslash-n pair through the fallback (the review's probe on 8aa2ab15a): a newline before a local in a
  // body scalac's reflection cannot read; the variant reads `@text("\\n")`, the two characters backslash and n.
  def blindEscape: Int =
    @text("\n") val v = Kinds.locals(outer = true)
    v
  // The delimiter pair through the fallback (the same review): a string holding a tag's delimiters before a local in a
  // body scalac's reflection cannot read; the variant reads `@text("a", "b], STRINGconst[c")`.
  def blindDelimit: Int =
    @text("a], STRINGconst[b", "c") val v = Kinds.locals(outer = true)
    v
  // The lambda parameter-name pair through the fallback (the same review): the refinement's method names its first
  // parameter `x`; the variant reads `Kinds.paramNames(first = false)`, the name `z`.
  def blindParams: Int =
    val other = Kinds.paramNames(first = true)
    val v = Kinds.locals(outer = true)
    v
  // The identity collision pair (the review's probe on 0324edfab): a return to the second `hop` (`hopS`) against one
  // to the method literally named `hop#2` (`hopLiteral`), both through a body scalac's reflection cannot read; the
  // variant reads the literal `hop#2`.
  @scala.annotation.targetName("hopI") def hop(x: Int): Int = x
  @scala.annotation.targetName("hopS") def hop(x: String): Int = x.length
  @scala.annotation.targetName("hopLiteral") def `hop#2`(x: Int): Int = x
  def blindCollision: Int =
    val other = Kinds.returnTo("hop", "hopS")
    val v = Kinds.locals(outer = true)
    v
  // The signed-name pair through the fallback (the review on b047da41b): a call to the first of `Overloads`' two `f`
  // overloads, whose parameter signatures flatten alike, inside a body scalac's reflection cannot read; the variant
  // reads `Kinds.signed(first = false)`, the call to the second.
  def blindSigned: Int =
    val call = Kinds.signed(first = true)
    val v = Kinds.locals(outer = true)
    v
  // A definition's annotations inside an annotation's argument: the variant reads `@tagged(8)`.
  val localAnnot: Int @shape({ @tagged(7) val x = 1; x }) = 1
  // A pattern binder's information: the variant reads `bind(false)`, `Int @tagged(8)` for the binder's `Int @tagged(7)`.
  val bindInfo = Kinds.bind(true)
  // A lambda's variances: the variant reads `variance(false)`, `F[-X]` for `F[+X]`.
  val variance = Kinds.variance(true)
  // An access boundary: the variant reads `within(false)`, `Outer.Product` for `Outer`.
  val within = Kinds.within(true)
  // The type trees an argument's ascriptions hold, a by-name parameter, the import selectors, named arguments, an
  // intersection (README.md, "Coverage": each kind reached from a root by a fixture).
  val typeTrees: Int @shape({
    import Flag.{inl as _, given}
    def byName(x: => Int): Int = x
    val annotated = (1: Int @shape(1))
    val selected = (Nil: scala.collection.immutable.List[Int])
    val projected = (??? : C#T)
    val refined = (null: C { type T = Int })
    val lambda = (Nil: ([X] =>> List[X])[Int])
    val matched = ("": Int match { case Int => String })
    val bound = (1: List[Int] match { case List[t] => t })
    val wild = (Nil: List[?])
    val wildPattern = (Nil: Any) match { case _: List[?] => 1; case _ => 0 }
    byName(1)
  }) = 1
  val named: Int @shape(x = 1) = 1
  def both(x: A1 & B1): Int = 1
  val built = Built.built
  // A named type's designator: the variant reads `designator(false)`, `DDerived`'s own `Inner` for the base's.
  private val designated = Kinds.designator(true)
  // A return's target, one of two overloads (the review's probe on 16b90a962): the variant reads `returnTarget(false)`.
  def jump(x: Int): Int = x
  def jump(x: String): Int = x.length
  val returned = Kinds.returnTarget(true)
  // Two overloads of one erasure apart by `@targetName` alone (the review's probe on c1be497fd): the variant reads
  // `returnTargetNamed(false)`.
  @scala.annotation.targetName("jumpInt") def jumpL(x: List[Int]): Int = 1
  @scala.annotation.targetName("jumpString") def jumpL(x: List[String]): Int = 2
  val returnedNamed = Kinds.returnTargetNamed(true)
  // A float's raw bits, a signalling NaN (the same review's probe): the variant reads its quiet twin.
  val nanBits = Kinds.floatBits(true)
  // The symbol reference's decisions (the ruling on 4c8ba8586), each with its variant in check.sh: overloads whose
  // target names are `super$jumpS` and `jumpS` (an ordinary method's target name is never unmangled), a return to one;
  @scala.annotation.targetName("super$jumpS") def jumpS(x: List[Int]): Int = 1
  @scala.annotation.targetName("jumpS") def jumpS(x: List[String]): Int = 2
  val returnedSuper = Kinds.returnTo("jumpS", "super$jumpS")
  // Two overloads apart by a `@targetName` written through an alias of its class (the review's probe on bd771417f),
  // a return to one: the variant reads `returnTo("jumpA", "jumpAS")`.
  type TN = scala.annotation.targetName
  @TN("jumpAI") def jumpA(x: List[Int]): Int = 1
  @TN("jumpAS") def jumpA(x: List[String]): Int = 2
  val returnedAliased = Kinds.returnTo("jumpA", "jumpAI")
  // a class member through an intersection prefix, found by the meet of both sides' members (the review's program);
  def andPrefix(x: AndA & AndB)(y: x.C): x.C = y
  // through a union prefix, by its join; through a refinement, by its parent;
  def orPrefix(x: OrA | OrB)(y: x.C): x.C = y
  def refinedPrefix(x: AndA { type T = Int })(y: x.C): x.C = y
  // a refinement's own member, which has no symbol: a name-designated reference;
  def nameDesignated(x: AnyRef { type T })(y: x.T): x.T = y
  // a non-method whose name has a signed name's shape, and the method it is shaped after.
  val `sigShaped(scala.Int)scala.Int`: Int = 1
  def sigShaped(x: Int): Int = x
  def sigShapedRef: Int @shape(Pickled.`sigShaped(scala.Int)scala.Int`) = 1
  // A refined method's selection: its result the refinement's (`String`), which the `TermRef` the selection is typed
  // by gives as its own type, read and not looked up (the ruling on dc2a3dfec's pass, finding 2).
  val refinedM: RefM { def f(x: Int): String } = null
  def refinedSelect: Int @shape(Pickled.refinedM.f(1)) = 1
  // A `summonFrom` in an annotation argument's inline definition (the review's probe on d54cf60be).
  val summoned: Int @shape({
    inline def pick: Int = scala.compiletime.summonFrom {
      case x: Int => x
      case _ => 0
    }
    0
  }) = 1
  // Every tree form an annotation argument holds, each observed through its extractor (README.md).
  val forms: Int @shape({
    import Flag.{flag as on, *}
    val l = 1
    val k: l.type = l
    var n = 0
    while n < 1 do n += 1
    def back(): Int = return n
    val f = (s: String) => s.length
    val r = try f("a") catch { case _: RuntimeException => 0 } finally ()
    val m = (n: Any) match
      case 1 | 2 => "one"
      case s: String if s.nonEmpty => s
      case x @ Some(_) => x.toString
      case _ => "other"
    if on then List(r, back(), Flag.inl(m.length), k) else (Nil: List[Int])
  }) = 1
  val recMember = (null: C { type T = Int; val b: T })

/** `This`, `Super` and `SuperType` in an annotation argument, typed in a class's own context. */
trait AndA { class C }
trait AndB
trait AndBC { class C }
trait OrP { class C }
trait OrA extends OrP
trait OrB extends OrP
trait OrQ extends OrP
// A companion's member, referred to from the class's own record: the root-local arm (isLocallyDefined).
class CompR { def m(x: CompR.Inner): CompR.Inner = x }
object CompR { class Inner }
object OtherR { class Inner }
class Sup extends Base:
  def selfAndSuper: Int @shape((this.hashCode, super.b)) = 1

trait RefM { def f(x: Int): Any }

// The fallback's quoted member names (the pass over 3693c7cff): a name holding an overload suffix's `#`, one holding
// an owner record's `:owner`, and a supplementary character, each body unreadable as the probes' are; and two unpaired
// surrogates, which scalac's name table encodes alike.
object Odd:
  def `f#2`: Int =
    val v = Kinds.locals(true)
    v
  def `f:owner`: Int =
    val v = Kinds.locals(true)
    v
  def `f😀`: Int =
    val v = Kinds.locals(true)
    v
  val a = "\ud800"
  val b = "\ud801"
