// Which of several applicable givens or extension methods is chosen.
package demo.ranking

import scala.annotation.targetName

trait Enc[A]:
  def name: String
final class EncImpl[A](val name: String) extends Enc[A]

trait Codec[A]:
  def name: String
final class CodecImpl[A](val name: String) extends Codec[A]

object Codec:
  given Codec[Boolean] = CodecImpl("boolean codec")
  given [A]: Codec[Set[A]] = CodecImpl("set codec")

trait LowPriorityEnc:
  given fromCodec[A](using c: Codec[A]): Enc[A] = EncImpl("fromCodec(" + c.name + ")")
  given lowInt: Enc[Int] = EncImpl("low priority int")

object Enc extends LowPriorityEnc:
  given string: Enc[String] = EncImpl("string")
  given int: Enc[Int] = EncImpl("int")
  given list[A](using e: Enc[A]): Enc[List[A]] = EncImpl("list(" + e.name + ")")
  given set[A](using e: Enc[A]): Enc[Set[A]] = EncImpl("set(" + e.name + ")")

// Two generic givens in one object: the one for the more specific type wins.
trait Dec[A]:
  def name: String
final class DecImpl[A](val name: String) extends Dec[A]
object Dec:
  given any[A]: Dec[A] = DecImpl("any")
  given option[A](using d: Dec[A]): Dec[Option[A]] = DecImpl("option(" + d.name + ")")
  given optionOfInt: Dec[Option[Int]] = DecImpl("option of int")

// The ladder of low-priority traits that tuple concatenation is typed with.
trait Concat[T, U, TU]:
  def name: String
final class ConcatImpl[T, U, TU](val name: String) extends Concat[T, U, TU]
trait ConcatLowest:
  given single[T, U]: Concat[T, U, (T, U)] = ConcatImpl("single")
trait ConcatLow extends ConcatLowest:
  given tuple2[A, B, U]: Concat[(A, B), U, (A, B, U)] = ConcatImpl("tuple2")
object Concat extends ConcatLow:
  given unitLeft[U]: Concat[Unit, U, U] = ConcatImpl("unitLeft")
  given unitRight[T]: Concat[T, Unit, T] = ConcatImpl("unitRight")

def concatName[T, U, TU](t: T, u: U)(using c: Concat[T, U, TU]): String = c.name

// Scala 3.7 prefers the more general of two givens whose types are related by subtyping.
trait Animal:
  def name: String
trait Dog extends Animal
final class AnimalImpl(val name: String) extends Animal
final class DogImpl(val name: String) extends Dog

trait Printer[-A]:
  def label: String
final class PrinterImpl[A](val label: String) extends Printer[A]

object Related:
  given animal: Animal = AnimalImpl("animal")
  given dog: Dog = DogImpl("dog")
  given animalPrinter: Printer[Animal] = PrinterImpl("printer of animals")
  given dogPrinter: Printer[Dog] = PrinterImpl("printer of dogs")

import Related.given

// A given without using parameters wins over one with them.
trait Tag[A]:
  def name: String
final class Named[A](val name: String) extends Tag[A]
object Tag:
  given needsEnc[A](using Enc[A]): Tag[List[A]] = Named("needs an Enc")
  given plain[A]: Tag[List[A]] = Named("plain")

class Scopes:
  given Tag[Int] = Named("class level")
  def inDef: String =
    given Tag[Int] = Named("def level")
    summon[Tag[Int]].name
  def inBlock: String =
    given Tag[Int] = Named("def level")
    val inner =
      given Tag[Int] = Named("block level")
      summon[Tag[Int]].name
    inner
  def parameter(using Tag[Int]): String = summon[Tag[Int]].name
  def fromClass: String = summon[Tag[Int]].name

object Holder:
  given Tag[String] = Named("from the enclosing object")
  def lookup: String = summon[Tag[String]].name
  object Inner:
    given Tag[String] = Named("from the inner object")
    def lookup: String = summon[Tag[String]].name

final case class Route[A](a: A)

extension [A](r: Route[A])
  def describe: String = "one"
extension (r: Route[Unit])
  @targetName("describeUnit") def describe: String = "unit"
extension [A, B](r: Route[(A, B)])
  @targetName("describePair") def describe: String = "pair"
extension [A, B, C](r: Route[(A, B, C)])
  @targetName("describeTriple") def describe: String = "triple"

extension [A](xs: List[A])
  def kind: String = "list"
extension [A](xs: List[Option[A]])
  @targetName("kindOptions") def kind: String = "list of options"
extension (xs: List[Option[Int]])
  @targetName("kindIntOptions") def kind: String = "list of int options"

extension [A](a: A) def tagged(n: Int): String = "any" + n
extension (s: String) def tagged(n: Int): String = "string" + n
extension (a: Animal) def tagged(n: Int): String = "animal" + n
extension (d: Dog) def tagged(n: Int): String = "dog" + n

// Where an opaque type is transparent the givens of its underlying type apply to it as well.
opaque type Email = String
object Email:
  def apply(s: String): Email = s
  given Enc[Email] = EncImpl("email")
  given Ordering[Email] = Ordering.by(e => e.length)

def encName[A](a: A)(using e: Enc[A]): String = e.name

// Extensions in lexical scope are tried before those of the receiver's companion, and the
// nearest scope that defines the name hides the outer ones.
final case class Crate(n: Int)
object Crate:
  extension (c: Crate) def tagged(n: Int): String = "crate" + n
  extension (c: Crate) def weigh: String = "crate of " + c.n

object Nearer:
  extension [A](a: A) def tagged(n: Int): String = "nearer" + n
  def test: String = "x".tagged(7)

@main def main(): Unit =
  println(summon[Enc[Int]].name)
  println(summon[Enc[String]].name)
  println(summon[Enc[Boolean]].name)
  println(summon[Enc[List[Int]]].name)
  println(summon[Enc[List[Boolean]]].name)
  println(summon[Enc[List[List[String]]]].name)
  println(summon[Enc[Set[Int]]].name)

  println(summon[Dec[String]].name)
  println(summon[Dec[Option[String]]].name)
  println(summon[Dec[Option[Int]]].name)
  println(summon[Dec[Option[Option[Int]]]].name)

  println(concatName(1, "a"))
  println(concatName((1, 2), "a"))
  println(concatName((), "a"))
  println(concatName(1, ()))

  println(summon[Animal].name)
  println(summon[Dog].name)
  println(summon[Printer[Dog]].label)
  println(summon[Printer[Animal]].label)

  println(summon[Tag[List[Int]]].name)

  val scopes = Scopes()
  println(scopes.inDef)
  println(scopes.inBlock)
  println(scopes.fromClass)
  println(scopes.parameter(using Named("explicit")))
  println(Holder.lookup)
  println(Holder.Inner.lookup)

  println(Route(1).describe)
  println(Route(()).describe)
  println(Route((1, "a")).describe)
  println(Route((1, "a", true)).describe)
  println(List(1).kind)
  println(List(Option("a")).kind)
  println(List(Option(1)).kind)
  println("x".tagged(1))
  println(1.tagged(2))
  println(DogImpl("rex").tagged(3))
  println(AnimalImpl("cat").tagged(4))
  println(Crate(1).tagged(5))
  println(Crate(2).weigh)
  println(Nearer.test)
  println(encName(Email("a@b")))
  println(encName("a@b"))
  println(List(Email("long@x"), Email("s@x")).sorted.map(_.toString))
