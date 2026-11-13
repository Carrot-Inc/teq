/*
rules = ["file:../../support/AnnotationNames.scala"]
*/
// The names in the arguments of an annotation are occurrences of the document, as scalac's SemanticDB has them
// (ExtractSemanticDB walks the annotations of a definition's symbol, of a constructor's and of a parameter's, and a
// type's annotation with its type tree): on a class, an object, a method, a value, a parameter, a type parameter, a
// secondary constructor, an enum case, a type alias, a given, a local value, and on a type in a signature, in a body
// and in a pattern; an argument that is a call, the annotation's own type argument, and an argument that reads the
// annotated definition's companion. A type's annotation sees what the type sees: an argument that names a parameter
// of the method, of a lambda or a pattern's binder is that parameter or binder, whatever else has the name.
package annotated

import scala.annotation.StaticAnnotation

enum Style:
  case Upper, Lower

object Limits:
  val low: Int = 1
  def scaled(n: Int): Int = n * 2

final class named(style: Style) extends StaticAnnotation
final class bound(n: Int) extends StaticAnnotation
final class tagged[T](label: String) extends StaticAnnotation

@named(Style.Upper) final case class First(a: Int) // assert: AnnotationNames.at

@bound(Limits.low) object Second // assert: AnnotationNames.at

class Holder[@bound(Limits.low) T](val held: T): // assert: AnnotationNames.at
  @named(Style.Lower) def method: Int = 1 // assert: AnnotationNames.at
  @bound(Limits.low) val value: Int = 1 // assert: AnnotationNames.at
  def taking(@bound(Limits.low) x: Int): Int = x // assert: AnnotationNames.at
  @bound(Limits.scaled(Limits.low)) def nested: Int = 2 // assert: AnnotationNames.at
  @tagged[Holder[Int]]("x") def typed: Int = 3 // assert: AnnotationNames.at
  def signature(x: Int @bound(Limits.low)): Int = x // assert: AnnotationNames.at
  def local: Int =
    @bound(Limits.low) val inner = 5 // assert: AnnotationNames.at
    val ascribed: Int @bound(Limits.low) = inner // assert: AnnotationNames.at
    ascribed
  inline def inlined(x: Int): Int =
    @bound(Limits.low) val within = x // assert: AnnotationNames.at
    val ascribed: Int @bound(Limits.low) = within // assert: AnnotationNames.at
    ascribed
  def matched(x: Any): Int = x match
    case n: (Int @bound(Limits.low)) => n // assert: AnnotationNames.at
    case _ => 0
  def context(using @bound(Limits.low) order: Ordering[Int]): Int = 1 // assert: AnnotationNames.at
  @bound(Limits.low) def this() = this(null.asInstanceOf[T]) // assert: AnnotationNames.at
  @bound(Holder.shared) def reading: Int = 3 // assert: AnnotationNames.at

object Holder:
  val shared: Int = 4

class Carrier(val size: Int):
  def taking(size: Int)(x: Int @bound(size)): Int = x // assert: AnnotationNames.at
  def giving(size: Int): Int @bound(size) = size // assert: AnnotationNames.at
  def inside(x: Int): Int =
    val each = (size: Int) => (size: Int @bound(size)) // assert: AnnotationNames.at
    each(x)
  def matched(x: Any): Int = x match
    case size: Int => (size: Int @bound(size)) // assert: AnnotationNames.at
    case _ => 0
  def local(size: Int): Int =
    val kept: Int @bound(size) = size // assert: AnnotationNames.at
    kept

enum Shade:
  @bound(Limits.low) case Dark // assert: AnnotationNames.at
  case Light

@bound(Limits.low) type Alias = Int // assert: AnnotationNames.at

object Uses:
  @bound(Limits.low) val first, second = 1 // assert: AnnotationNames.at
  @bound(Limits.low) given ordering: Ordering[Shade] = Ordering.by(_.ordinal) // assert: AnnotationNames.at
  val called: Int = Holder[Int](1).inlined(2)
