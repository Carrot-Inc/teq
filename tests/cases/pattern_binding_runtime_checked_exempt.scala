// teq: --werror
// The forms `.runtimeChecked` exempts a pattern definition through: the `RuntimeCheckedAnnot` on the
// typed scrutinee's type, which `Checking.checkIrrefutable` reads (`pt.hasAnnotation`), carried as
// dotty's typing carries an annotated type. An ascription to the annotated type keeps it (written
// whole or as the annotation alone), an `if`'s or a match's branches that each give the same
// annotated type join to it, a transparent inline wrapper's expansion has its body's (the bindings
// avoided, as `TypeAssigner` types an `Inlined`), a plain inline method declaring it gives it too.
package patternbindingruntimecheckedexempt

import scala.annotation.internal.RuntimeChecked
import scala.annotation.internal.{RuntimeChecked as RC}

transparent inline def checked(x: Option[Int]) = x.runtimeChecked
inline def declared(x: Option[Int]): Option[Int] @scala.annotation.internal.RuntimeChecked = x
extension (s: String) def sized: Option[Int] @RuntimeChecked = Some(s.length)
// The annotation renamed by its import is the same class, resolved by the typer.
def checkedRC(o: Option[Int]): Option[Int] @RC = o

object Main:
  def main(args: Array[String]): Unit =
    val opt = Option(7)
    val Some(a) = (opt: @scala.annotation.internal.RuntimeChecked)
    val Some(b) = (opt: Option[Int] @scala.annotation.internal.RuntimeChecked)
    val Some(c) = (if args.isEmpty then opt.runtimeChecked else opt.runtimeChecked)
    val Some(d) = (args.length match { case 0 => opt.runtimeChecked; case _ => opt.runtimeChecked })
    val Some(e) = { println("once"); Option(5).runtimeChecked }
    val Some(f) = checked(opt)
    val Some(g) = checked(Option(6))
    val Some(h) = declared(Option(4))
    val (Some(i), j) = (opt, 3).runtimeChecked
    val Some(k) = (Option(1): @RuntimeChecked)
    println(List(a, b, c, d, e, f, g, h, i, j, k).mkString(" "))
    val t: Int = checked(Option(2)) match { case Some(k) => k }
    println(t)
    // The result is the receiver's singleton as `Predef`'s (`x.type @RuntimeChecked`), a constant's
    // its constant type (the std's body bound over the constant as dotty's inliner binds it).
    val same: opt.type = opt.runtimeChecked
    val five: 5 = 5.runtimeChecked
    val word: "eleven" = "eleven".runtimeChecked
    println(s"${same eq opt} $five $word")
    // Joins as `TypeComparer.lub` keeps an annotated operand: an extension's `Option[Int]
    // @RuntimeChecked` of two receivers, `opt` then `opt.runtimeChecked` (the later singleton), a
    // finalizer's `try` (the body's type), a handler's annotated result, an annotated ascription
    // first, a branch that throws.
    val cond = args.isEmpty
    val Some(j1) = if cond then "ab".sized else "abc".sized
    val Some(j2) = if cond then opt else opt.runtimeChecked
    val Some(j3) = try opt.runtimeChecked finally println("finally")
    val Some(j4) = try opt.runtimeChecked catch { case _: Exception => opt.runtimeChecked }
    val Some(j5) = try opt catch { case _: Exception => opt.runtimeChecked }
    val Some(j6) = if cond then (opt: Option[Int] @RuntimeChecked) else (Option(9): Option[Int])
    val Some(j7) = if cond then opt.runtimeChecked else throw new IllegalStateException("never")
    val Some(j8) = args.length match { case 0 => "ab".sized; case _ => "abc".sized }
    val Some(j9) = if cond then "ab".sized else opt.runtimeChecked
    // Two receivers' union, then an annotated supertype of both parts, which subsumes it.
    val other = Option(3)
    val Some(j10) = args.length match
      case 0 => opt.runtimeChecked
      case 1 => other.runtimeChecked
      case _ => (opt: Option[Int] @RuntimeChecked)
    println(List(j1, j2, j3, j4, j5, j6, j7, j8, j9, j10).mkString(" "))
    val Some(r1) = (Option(1): @RC)
    val Some(r2) = checkedRC(Option(2))
    val Some(r3) = (Option(3): Option[Int] @RC)
    println(r1 + r2 + r3)
