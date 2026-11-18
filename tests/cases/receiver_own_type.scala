// A view or an extension method applied to a receiver written as a plain path or a literal token takes its own type,
// not the type its member was looked up in: a union joined, a literal or a singleton widened for the lookup keep
// their own type for the view's or the extension's variable, which is then instantiated as any is (widened where
// nothing bounds it). A path to a term (a val, var, lazy val, parameterless def or by-name parameter) meets the
// variable as that reference and widens to its declared type, an alias of a union kept by name. Any other receiver
// (an ascription, an application, a block, a folded constant) meets it as the lookup's type. Each use's type
// argument is held by an invariant `Cell[T]`.
import scala.language.implicitConversions

type Stage = "draft" | "final"

final class Cell[A](val a: A):
  def ->>(n: Int): Cell[A] = this
  override def toString = s"Cell($a)"
final class Lift[A](private val a: A) extends AnyVal:
  def lifted: Cell[A] = Cell(a)
implicit def toLift[A](a: A): Lift[A] = new Lift(a)
extension [A](a: A) def boxed: Cell[A] = Cell(a)
implicit def toCell[A](a: A): Cell[A] = Cell(a)

sealed trait Animal
final class Cat extends Animal:
  override def toString = "Cat"
final class Dog extends Animal:
  override def toString = "Dog"
type Pet = Cat | Dog

object Opaque:
  opaque type Code = String
  val code: Code = "c"
  def codeDef: Code = "c"
trait Holder:
  type T
  val t: T
  def tDef: T

object Receivers:
  val stage: Stage = "final"
  val single: "final" = "final"
  var mutable: Stage = "final"
  lazy val later: Stage = "final"
  def method: Stage = "final"
  final val constant = "final"
  inline val inlined = "final"
  def same[A](a: A): A = a
  def byName(x: => Stage): List[Cell[Stage]] =
    val v01 = x.lifted; val w01: Cell[Stage] = v01
    val e01 = x.boxed; val x01: Cell[Stage] = e01
    val i01 = x ->> 1; val j01: Cell[Stage] = i01
    List(w01, x01, j01)
  val v02 = stage.lifted; val w02: Cell[Stage] = v02
  val v03 = single.lifted; val w03: Cell[String] = v03
  val v04 = "final".lifted; val w04: Cell[String] = v04
  val v05 = ("final": Stage).lifted; val w05: Cell[String] = v05
  val v06 = same("final").lifted; val w06: Cell[String] = v06
  val v07 = { "final" }.lifted; val w07: Cell[String] = v07
  val v08 = mutable.lifted; val w08: Cell[Stage] = v08
  val v09 = later.lifted; val w09: Cell[Stage] = v09
  val v10 = method.lifted; val w10: Cell[Stage] = v10
  val v11 = constant.lifted; val w11: Cell[String] = v11
  val v12 = inlined.lifted; val w12: Cell[String] = v12
  val v13 = ("fi" + "nal").lifted; val w13: Cell[String] = v13
  val e02 = stage.boxed; val x02: Cell[Stage] = e02
  val e03 = single.boxed; val x03: Cell[String] = e03
  val e04 = "final".boxed; val x04: Cell[String] = e04
  val e05 = ("final": Stage).boxed; val x05: Cell[String] = e05
  val e06 = same("final").boxed; val x06: Cell[String] = e06
  val e07 = { "final" }.boxed; val x07: Cell[String] = e07
  val e08 = mutable.boxed; val x08: Cell[Stage] = e08
  val e09 = later.boxed; val x09: Cell[Stage] = e09
  val e10 = method.boxed; val x10: Cell[Stage] = e10
  val e11 = constant.boxed; val x11: Cell[String] = e11
  val e12 = inlined.boxed; val x12: Cell[String] = e12
  val e13 = ("fi" + "nal").boxed; val x13: Cell[String] = e13
  val i02 = stage ->> 1; val j02: Cell[Stage] = i02
  val i03 = single ->> 1; val j03: Cell[String] = i03
  val i04 = "final" ->> 1; val j04: Cell[String] = i04
  val i05 = ("final": Stage) ->> 1; val j05: Cell[String] = i05
  val i06 = same("final") ->> 1; val j06: Cell[String] = i06
  val i07 = { "final" } ->> 1; val j07: Cell[String] = i07
  val i08 = mutable ->> 1; val j08: Cell[Stage] = i08
  val i09 = later ->> 1; val j09: Cell[Stage] = i09
  val i10 = method ->> 1; val j10: Cell[Stage] = i10
  val i11 = constant ->> 1; val j11: Cell[String] = i11
  val i12 = inlined ->> 1; val j12: Cell[String] = i12
  val i13 = ("fi" + "nal") ->> 1; val j13: Cell[String] = i13
  def shown: List[Any] = byName("draft") ++ List(w02, w03, w04, w05, w06, w07, w08, w09, w10, w11, w12, w13, x02, x03, x04, x05, x06, x07, x08, x09, x10, x11, x12, x13, j02, j03, j04, j05, j06, j07, j08, j09, j10, j11, j12, j13)

object OtherDeclared:
  val single: "final" = "final"
  var singleVar: "final" = "final"
  def singleDef: "final" = "final"
  val union: "draft" | "final" = "final"
  var unionVar: "draft" | "final" = "final"
  def unionDef: "draft" | "final" = "final"
  def unionByName(x: => "draft" | "final"): (Cell[String], Cell[String], Cell[String]) = (x.lifted, x.boxed, x ->> 1)
  def param[T](t: T): (Cell[T], Cell[T], Cell[T]) = (t.lifted, t.boxed, t ->> 1)
  def paramBounded[T <: "draft" | "final"](t: T): (Cell[T], Cell[T], Cell[T]) = (t.lifted, t.boxed, t ->> 1)
  def paramByName[T](t: => T): (Cell[T], Cell[T], Cell[T]) = (t.lifted, t.boxed, t ->> 1)
  val o1 = Opaque.code.lifted
  val w_o1: Cell[Opaque.Code] = o1
  val o2 = Opaque.code.boxed
  val w_o2: Cell[Opaque.Code] = o2
  val o3 = Opaque.code ->> 1
  val w_o3: Cell[Opaque.Code] = o3
  val o4 = Opaque.codeDef.lifted
  val w_o4: Cell[Opaque.Code] = o4
  val o5 = Opaque.codeDef.boxed
  val w_o5: Cell[Opaque.Code] = o5
  def dep(h: Holder): (Cell[h.T], Cell[h.T], Cell[h.T], Cell[h.T], Cell[h.T]) = (h.t.lifted, h.t.boxed, h.t ->> 1, h.tDef.lifted, h.tDef.boxed)
  def depFixed(h: Holder { type T = "draft" | "final" }): (Cell["draft" | "final"], Cell["draft" | "final"], Cell["draft" | "final"], Cell["draft" | "final"]) = (h.t.lifted, h.t.boxed, h.t ->> 1, h.tDef.lifted)
  val s1 = single.lifted
  val w_s1: Cell[String] = s1
  val s2 = single.boxed
  val w_s2: Cell[String] = s2
  val s3 = single ->> 1
  val w_s3: Cell[String] = s3
  val s4 = singleVar.lifted
  val w_s4: Cell[String] = s4
  val s5 = singleVar.boxed
  val w_s5: Cell[String] = s5
  val s6 = singleDef.lifted
  val w_s6: Cell[String] = s6
  val s7 = singleDef.boxed
  val w_s7: Cell[String] = s7
  val u1 = union.lifted
  val w_u1: Cell[String] = u1
  val u2 = union.boxed
  val w_u2: Cell[String] = u2
  val u3 = union ->> 1
  val w_u3: Cell[String] = u3
  val u4 = unionVar.lifted
  val w_u4: Cell[String] = u4
  val u5 = unionVar.boxed
  val w_u5: Cell[String] = u5
  val u6 = unionDef.lifted
  val w_u6: Cell[String] = u6
  val u7 = unionDef.boxed
  val w_u7: Cell[String] = u7
  val u8 = unionDef ->> 1
  val w_u8: Cell[String] = u8
  def shown: List[Any] = List(unionByName("draft"), param(1), paramBounded("draft"), paramByName(2.5), dep(new Holder { type T = Int; val t = 3; def tDef = 4 }), depFixed(new Holder { type T = "draft" | "final"; val t = "final"; def tDef = "draft" }), w_o1, w_o2, w_o3, w_o4, w_o5, w_s1, w_s2, w_s3, w_s4, w_s5, w_s6, w_s7, w_u1, w_u2, w_u3, w_u4, w_u5, w_u6, w_u7, w_u8)

object ClassUnion:
  val pet: Pet = Cat()
  def petDef: Pet = Cat()
  val v1 = (pet: Pet).lifted
  val w_v1: Cell[Cat | Dog] = v1
  val v2 = (pet: Cat | Dog).lifted
  val w_v2: Cell[Cat | Dog] = v2
  val v3 = pet.lifted
  val w_v3: Cell[Cat | Dog] = v3
  val v4 = petDef.lifted
  val w_v4: Cell[Cat | Dog] = v4
  val e1 = (pet: Pet).boxed
  val w_e1: Cell[Cat | Dog] = e1
  val e2 = (pet: Cat | Dog).boxed
  val w_e2: Cell[Cat | Dog] = e2
  val e3 = pet.boxed
  val w_e3: Cell[Cat | Dog] = e3
  val e4 = petDef.boxed
  val w_e4: Cell[Cat | Dog] = e4
  def plain[A](a: A): Cell[A] = Cell(a)
  val p1 = plain(pet: Pet)
  val w_p1: Cell[Cat | Dog] = p1
  val p2 = plain(pet: Cat | Dog)
  val w_p2: Cell[Cat | Dog] = p2
  def shown: List[Any] = List(w_v1, w_v2, w_v3, w_v4, w_e1, w_e2, w_e3, w_e4, w_p1, w_p2)

object ExpectedBound:
  val stage: Stage = "final"
  val single: "final" = "final"
  var mutable: Stage = "final"
  lazy val later: Stage = "final"
  def method: Stage = "final"
  final val constant = "final"
  inline val inlined = "final"
  def byName(x: => Stage): List[Cell[Stage]] = List(x.lifted, x.boxed, x ->> 1)
  val v02: Cell[Stage] = stage.lifted
  val e02: Cell[Stage] = stage.boxed
  val i02: Cell[Stage] = stage ->> 1
  val v03: Cell[Stage] = single.lifted
  val e03: Cell[Stage] = single.boxed
  val i03: Cell[Stage] = single ->> 1
  val v04: Cell[Stage] = "final".lifted
  val e04: Cell[Stage] = "final".boxed
  val i04: Cell[Stage] = "final" ->> 1
  val e06: Cell[Stage] = { "final" }.boxed
  val v07: Cell[Stage] = mutable.lifted
  val e07: Cell[Stage] = mutable.boxed
  val i07: Cell[Stage] = mutable ->> 1
  val v08: Cell[Stage] = later.lifted
  val e08: Cell[Stage] = later.boxed
  val i08: Cell[Stage] = later ->> 1
  val v09: Cell[Stage] = method.lifted
  val e09: Cell[Stage] = method.boxed
  val i09: Cell[Stage] = method ->> 1
  val v10: Cell[Stage] = constant.lifted
  val e10: Cell[Stage] = constant.boxed
  val i10: Cell[Stage] = constant ->> 1
  val v11: Cell[Stage] = inlined.lifted
  val e11: Cell[Stage] = inlined.boxed
  val i11: Cell[Stage] = inlined ->> 1
  val e12: Cell[Stage] = ("fi" + "nal").boxed
  def shown: List[Any] = byName("draft") ++ List(v02, e02, i02, v03, e03, i03, v04, e04, i04, e06, v07, e07, i07, v08, e08, i08, v09, e09, i09, v10, e10, i10, v11, e11, i11, e12)

// An opaque type in its own scope is its underlying type to a lookup, and to an infix extension on it.
extension [A](a: A) def +++(b: A): Cell[A] = Cell(b)
object OpaqueInside:
  opaque type Meters = Double
  def mk(d: Double): Meters = d
  def twice(m: Meters) = m +++ m
object OpaqueOutside:
  val w: Cell[Double] = OpaqueInside.twice(OpaqueInside.mk(4.0))
  def shown: List[Any] = List(w.a == 4.0)

@main def run(): Unit =
  println(Receivers.shown.mkString(" "))
  println(OtherDeclared.shown.mkString(" "))
  println(ClassUnion.shown.mkString(" "))
  println(ExpectedBound.shown.mkString(" "))
  println(OpaqueOutside.shown.mkString(" "))
