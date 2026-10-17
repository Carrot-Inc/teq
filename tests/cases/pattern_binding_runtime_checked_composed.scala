// teq: --werror
// An annotated type composed as dotty's typing composes it (`AnnotatedType` kept on the type):
// a join's operand that is itself a join keeps its union of singletons, which a later annotated
// supertype subsumes in either order (`TypeComparer.lub`, TypeComparer.scala 2572-2614); a
// transparent expansion's type is its body's with the bindings avoided (TypeAssigner.scala
// 425-429), the argument's path through an ordinary, an `inline` and a by-name parameter alike,
// so two expansions over one receiver are one singleton. An annotation alone over an expression
// annotates the type it was assigned (`typedAnnotated`'s `arg1.tpe.widenIfUnstable`): an
// ascription's written type, which the plain branch's singleton conforms to, in either order. A
// component whose type carries the annotation is exempt as each component is tested
// (Checking.scala 1141-1144): an irrefutable extractor's `Tuple1[Option[Int] @RuntimeChecked]`, an
// alias annotated where it is named (`Tuple1[Plain @RuntimeChecked]`, an alias of an annotated type
// annotated again); a sequence exempt through `.runtimeChecked` on the whole right side.
package patternbindingruntimecheckedcomposed

import scala.annotation.internal.RuntimeChecked

object Wrapped:
  def unapply(x: Int): Tuple1[Option[Int] @RuntimeChecked] = Tuple1(Some(x))

type Plain = Option[Int]
type Checked = Option[Int] @RuntimeChecked
object PlainAnnotated:
  def unapply(x: Int): Tuple1[Plain @RuntimeChecked] = Tuple1(Some(x))
object CheckedAnnotated:
  def unapply(x: Int): Tuple1[Checked @RuntimeChecked] = Tuple1(Some(x))

transparent inline def wrap(x: Option[Int]) = x.runtimeChecked
transparent inline def wrapInline(inline x: Option[Int]) = x.runtimeChecked
transparent inline def wrapByName(x: => Option[Int]) = x.runtimeChecked

object Main:
  def main(args: Array[String]): Unit =
    val a = Option(7)
    val b = Option(8)
    val Some(n1) = if args.length == 0 then (if args.length == 1 then a.runtimeChecked else b.runtimeChecked) else (a: Option[Int] @RuntimeChecked)
    val Some(n2) = if args.length != 0 then (a: Option[Int] @RuntimeChecked) else (if args.length == 1 then a.runtimeChecked else b.runtimeChecked)
    val Some(n3) = if args.length == 0 then wrap(a) else wrap(a)
    val Some(n4) = if args.length == 0 then wrapInline(a) else wrapInline(a)
    val Some(n5) = if args.length == 0 then wrapByName(a) else wrapByName(a)
    val Some(n6) = if args.length == 0 then ((a: Option[Int]): @RuntimeChecked) else a
    val Some(n7) = if args.length != 0 then a else ((a: Option[Int]): @RuntimeChecked)
    val Some(n8) = if args.length == 0 then ({ (a: Option[Int]) }: @RuntimeChecked) else a
    val Wrapped(Some(n9)) = 9
    val PlainAnnotated(Some(n10)) = 10
    val CheckedAnnotated(Some(n11)) = 11
    val listed: List[Option[Int] @RuntimeChecked] = List(Some(12))
    val List(Some(n12)) = listed.runtimeChecked
    println(List(n1, n2, n3, n4, n5, n6, n7, n8, n9, n10, n11, n12).mkString(" "))
