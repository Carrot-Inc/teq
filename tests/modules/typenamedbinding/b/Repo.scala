package tnbb

import scala.compiletime.*
import scala.deriving.Mirror
import tnba.*
import tnba.Keys.*

// A derivation in doobie's shape (`WritePlatform.derivedImpl`): the mirror an inline method takes
// is bound for the non-inline `derivedImpl`, whose body reads it nowhere but names it in a val's
// written type. `Inliner.dropUnusedDefs` keeps that binding (`countRefs` meets the `Ident` of `m`
// in the type tree), from the companion compiled with the program and read from its products alike.
trait Write[A]:
  def write(a: A): List[String]

object Write:
  given Write[Int] with
    def write(a: Int): List[String] = List(a.toString)
  given Write[String] with
    def write(a: String): List[String] = List(a)
  given Write[Key] with
    def write(a: Key): List[String] = List("key:" + a.value)

  final class Composite[A](parts: List[Write[Any]], items: A => List[Any]) extends Write[A]:
    def write(a: A): List[String] = parts.zip(items(a)).flatMap((w, x) => w.write(x))

  inline def derived[A <: Product](using inline m: Mirror.ProductOf[A]): Write[A] = derivedImpl[A](using m)

  private inline def derivedImpl[A <: Product](using m: Mirror.ProductOf[A]): Write[A] =
    inline erasedValue[m.MirroredElemTypes] match
      case _: EmptyTuple => error("no fields")
      case _ =>
        val writes: Tuple.Map[m.MirroredElemTypes, Write] = summonAll[Tuple.Map[m.MirroredElemTypes, Write]]
        new Composite[A](writes.toList.asInstanceOf[List[Write[Any]]], (a: A) => a.productIterator.toList)

object Repo:
  given Write[SpaceId] = Write.derived
  def update(id: SpaceId): List[String] = Write.derived[SpaceId].write(id)
  def named(n: Named): List[String] = Write.derived[Named].write(n)

@main def run(): Unit =
  println(Repo.update(SpaceId(3)))
  println(Repo.named(Named(Key("k"), SpaceId(4))))
  println(List(SpaceId(2), SpaceId(1)).sorted)
