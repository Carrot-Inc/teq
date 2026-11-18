// A view's or an extension method's type variable meets a receiver written as a plain path (an identifier or a
// selection naming a val, a var, a lazy val, a def, a by-name parameter, bare or in parentheses, a pattern's binder)
// as that reference, its declared type's alias kept, and the given for it is chosen; any other form (an ascription,
// an annotated or singleton-ascribed reference, an application, an applied lambda, a polymorphic def given its type
// argument, a named tuple's field, `identity`, `locally`, a `match`) is met as its type widened. Each form over a val,
// a var, a lazy val, a def and a by-name parameter, through an extension and through a view.
import scala.language.implicitConversions
trait Label[A]:
  def text: String
type Stage = "draft" | "final"
given Label[String] with
  def text = "wide"
given Label[Stage] with
  def text = "narrow"
extension [A](a: A)
  def label(using l: Label[A]): String = l.text
  def self: A = a
final class Viewed[A](a: A)(using l: Label[A]):
  def which: String = l.text
implicit def toViewed[A](a: A)(using l: Label[A]): Viewed[A] = Viewed(a)
given Int = 3

object Family:
  val sv: Stage = "final"
  var vv: Stage = "final"
  lazy val lz: Stage = "final"
  def dv: Stage = "final"
  def mp(): Stage = "final"
  def mpoly[T]: Stage = "final"
  def mu(using n: Int): Stage = "final"
  inline def minl: Stage = "final"
  transparent inline def mtin: Stage = "final"
  def run(bn: => Stage): Unit =
    println("sv plain: " + sv.label + " " + sv.which)
    println("sv parens: " + (sv).label + " " + (sv).which)
    println("sv ascribed: " + (sv: Stage).label + " " + (sv: Stage).which)
    println("sv annotated-typed: " + (sv: Stage @unchecked).label + " " + (sv: Stage @unchecked).which)
    println("sv identity: " + identity(sv).label + " " + identity(sv).which)
    println("sv locally: " + locally(sv).label + " " + locally(sv).which)
    println("sv beta: " + (() => sv)().label + " " + (() => sv)().which)
    println("sv lambda-arg: " + ((x: Stage) => x)(sv).label + " " + ((x: Stage) => x)(sv).which)
    println("sv match: " + (sv match { case x => x }).label + " " + (sv match { case x => x }).which)
    println("sv self: " + sv.self.label + " " + sv.self.which)
    println("sv named-field: " + (st = sv, n = 1).st.label + " " + (st = sv, n = 1).st.which)
    println("%s pattern: " + (sv match { case x @ _ => x.label + " " + x.which }))
    println("vv plain: " + vv.label + " " + vv.which)
    println("vv parens: " + (vv).label + " " + (vv).which)
    println("vv ascribed: " + (vv: Stage).label + " " + (vv: Stage).which)
    println("vv annotated: " + (vv: @unchecked).label + " " + (vv: @unchecked).which)
    println("vv annotated-typed: " + (vv: Stage @unchecked).label + " " + (vv: Stage @unchecked).which)
    println("vv identity: " + identity(vv).label + " " + identity(vv).which)
    println("vv locally: " + locally(vv).label + " " + locally(vv).which)
    println("vv beta: " + (() => vv)().label + " " + (() => vv)().which)
    println("vv lambda-arg: " + ((x: Stage) => x)(vv).label + " " + ((x: Stage) => x)(vv).which)
    println("vv match: " + (vv match { case x => x }).label + " " + (vv match { case x => x }).which)
    println("vv self: " + vv.self.label + " " + vv.self.which)
    println("vv named-field: " + (st = vv, n = 1).st.label + " " + (st = vv, n = 1).st.which)
    println("%s pattern: " + (vv match { case x @ _ => x.label + " " + x.which }))
    println("lz plain: " + lz.label + " " + lz.which)
    println("lz parens: " + (lz).label + " " + (lz).which)
    println("lz ascribed: " + (lz: Stage).label + " " + (lz: Stage).which)
    println("lz annotated-typed: " + (lz: Stage @unchecked).label + " " + (lz: Stage @unchecked).which)
    println("lz identity: " + identity(lz).label + " " + identity(lz).which)
    println("lz locally: " + locally(lz).label + " " + locally(lz).which)
    println("lz beta: " + (() => lz)().label + " " + (() => lz)().which)
    println("lz lambda-arg: " + ((x: Stage) => x)(lz).label + " " + ((x: Stage) => x)(lz).which)
    println("lz match: " + (lz match { case x => x }).label + " " + (lz match { case x => x }).which)
    println("lz self: " + lz.self.label + " " + lz.self.which)
    println("lz named-field: " + (st = lz, n = 1).st.label + " " + (st = lz, n = 1).st.which)
    println("%s pattern: " + (lz match { case x @ _ => x.label + " " + x.which }))
    println("dv plain: " + dv.label + " " + dv.which)
    println("dv parens: " + (dv).label + " " + (dv).which)
    println("dv ascribed: " + (dv: Stage).label + " " + (dv: Stage).which)
    println("dv annotated: " + (dv: @unchecked).label + " " + (dv: @unchecked).which)
    println("dv annotated-typed: " + (dv: Stage @unchecked).label + " " + (dv: Stage @unchecked).which)
    println("dv identity: " + identity(dv).label + " " + identity(dv).which)
    println("dv locally: " + locally(dv).label + " " + locally(dv).which)
    println("dv beta: " + (() => dv)().label + " " + (() => dv)().which)
    println("dv lambda-arg: " + ((x: Stage) => x)(dv).label + " " + ((x: Stage) => x)(dv).which)
    println("dv match: " + (dv match { case x => x }).label + " " + (dv match { case x => x }).which)
    println("dv self: " + dv.self.label + " " + dv.self.which)
    println("dv named-field: " + (st = dv, n = 1).st.label + " " + (st = dv, n = 1).st.which)
    println("%s pattern: " + (dv match { case x @ _ => x.label + " " + x.which }))
    println("bn plain: " + bn.label + " " + bn.which)
    println("bn parens: " + (bn).label + " " + (bn).which)
    println("bn ascribed: " + (bn: Stage).label + " " + (bn: Stage).which)
    println("bn annotated: " + (bn: @unchecked).label + " " + (bn: @unchecked).which)
    println("bn annotated-typed: " + (bn: Stage @unchecked).label + " " + (bn: Stage @unchecked).which)
    println("bn identity: " + identity(bn).label + " " + identity(bn).which)
    println("bn locally: " + locally(bn).label + " " + locally(bn).which)
    println("bn beta: " + (() => bn)().label + " " + (() => bn)().which)
    println("bn lambda-arg: " + ((x: Stage) => x)(bn).label + " " + ((x: Stage) => x)(bn).which)
    println("bn match: " + (bn match { case x => x }).label + " " + (bn match { case x => x }).which)
    println("bn self: " + bn.self.label + " " + bn.self.which)
    println("bn named-field: " + (st = bn, n = 1).st.label + " " + (st = bn, n = 1).st.which)
    println("%s pattern: " + (bn match { case x @ _ => x.label + " " + x.which }))
    println("mp(): " + mp().label + " " + mp().which)
    println("mpoly[Int]: " + mpoly[Int].label + " " + mpoly[Int].which)
    println("mpoly: " + mpoly.label + " " + mpoly.which)
    println("mu: " + mu.label + " " + mu.which)
    println("minl: " + minl.label + " " + minl.which)
    println("mtin: " + mtin.label + " " + mtin.which)

@main def run(): Unit = Family.run("draft")
