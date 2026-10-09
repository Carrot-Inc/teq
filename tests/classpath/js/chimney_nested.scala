// jars: scala-library chimney chimney-macro-commons scala-collection-compat scala-java-time
//> using dep io.scalaland::chimney:1.11.0
// chimney 1.11.0 deriving through nested case classes and collections from its jar: a field whose
// type is another pair of case classes, `List` into `Vector`, `Option` of a nested class, a `Map`
// of nested values, a value wrapped into `Option`, and a sealed trait into a sealed trait of the
// same case names.
// scala-java-time is on the class path, as the application's is: chimney's macros time their
// derivation with `java.time.Instant.now()`, which the interpreter runs from that jar.
package chimneynested

import io.scalaland.chimney.dsl.*

final case class InnerA(x: Int, y: String)
final case class InnerB(x: Int, y: String)
sealed trait KindA
object KindA:
  case object Small extends KindA
  final case class Big(size: Int) extends KindA
sealed trait KindB
object KindB:
  case object Small extends KindB
  final case class Big(size: Int) extends KindB

final case class OuterA(inner: InnerA, list: List[InnerA], opt: Option[InnerA], byKey: Map[String, InnerA],
                        plain: Int, kinds: List[KindA])
final case class OuterB(inner: InnerB, list: Vector[InnerB], opt: Option[InnerB], byKey: Map[String, InnerB],
                        plain: Option[Int], kinds: Vector[KindB])

@main def main(): Unit =
  val a = OuterA(InnerA(1, "a"), List(InnerA(2, "b"), InnerA(3, "c")), Some(InnerA(4, "d")),
    Map("k" -> InnerA(5, "e")), 6, List(KindA.Small, KindA.Big(9)))
  println(a.transformInto[OuterB])
  println(a.copy(opt = None, list = Nil).transformInto[OuterB])
  println(List(KindA.Big(1), KindA.Small).transformInto[List[KindB]])
