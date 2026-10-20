//> using platform js
//> using dep io.github.cquiroz::scala-java-time::2.7.0
// jars: scala-java-time
// A Scala enum that extends java.lang.Enum: each of its values is an instance of `$new`'s class, which dotty
// gives Mirror.Singleton as any enum's (`DesugarEnums.enumValueCreator`'s `ExtendsSingletonMirror`,
// `SyntheticMembers.addMirrorSupport`). scala-java-time's `DayOfWeek` and `Month` are such enums, read from the
// jar's TASTy, whose compiled value classes carry the parent (`DayOfWeek$$anon$1 implements EnumValue,
// Mirror$Singleton`). scalac prints `true` on every line. teq, on master too, gives the parent to no value of
// a source enum (`mirror_parent`'s `is_java_enum_value`) and to one value of each enum read from TASTy, the
// one the program reaches first (`SUNDAY` and `JANUARY` here, before the enum's parents are read). Each value
// is tested as `Any`: on the enum's own type scalac folds the test to `false` at compile time.
import java.time.{DayOfWeek, Month}
import scala.deriving.Mirror
enum E extends java.lang.Enum[E]:
  case A, B, C
enum F:
  case D, G
def singleton(v: Any): Boolean = v.isInstanceOf[Mirror.Singleton]
@main def run(): Unit =
  println(s"DayOfWeek.SUNDAY ${singleton(DayOfWeek.SUNDAY)}")
  println(s"Month.JANUARY ${singleton(Month.JANUARY)}")
  for e <- E.values do println(s"E.$e ${singleton(e)}")
  for f <- F.values do println(s"F.$f ${singleton(f)}")
  for d <- DayOfWeek.values do println(s"DayOfWeek.$d ${singleton(d)}")
  for m <- Month.values do println(s"Month.$m ${singleton(m)}")
