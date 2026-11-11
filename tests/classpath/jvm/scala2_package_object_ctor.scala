// jars: scala-library scala2-lib
// Scala 2.13 traits' constructors are no members a class inherits (tests/support/scala2_lib/timecodecs.scala, the shape
// of kantan.codecs-java8's `kantan.codecs.strings.java8` package object over `TimeCodecCompanion`): a constructor meets
// no inherited member (`ClassDenotation#addInherited`, SymDenotations.scala:2229) and matches no inherited symbol
// (`memberCanMatchInheritedSymbols`, 1442-1443). So a library trait mixing two unrelated traits joins no entry of their
// `<init>`s, and neither a package object over it nor a class mixing the two traits meets an inherited `<init>`.
// tr/integration aborted entering the package object, at `mark_members_meeting_inherited` ("no entry found for key"),
// the joined `<init>` standing in the trait's members while the scope keeps a class's own `<init>` out of them.
import scala2lib.timecodecs.*
import scala2lib.timecodecs.time.*

class Both extends TimeDecoders[String, Int] with TimeEncoders[String]:
  def decode(text: String): Either[Int, String] = Left(text.length)

@main def run(): Unit =
  println(codec("iso"))
  println(decode("x"))
  val b = Both()
  println(b.decoder("d") + " " + b.encoder("e") + " " + b.decode("abc"))
