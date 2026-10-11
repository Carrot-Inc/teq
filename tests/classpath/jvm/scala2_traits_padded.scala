// jars: scala-library scala2-traits-padded
// Each class of a jar is Scala 2's or not by its own class file, as dotty's `ClassfileParser` reads each class's
// metadata (`unpickleOrParseInnerClasses`): the scala2-traits jar with Java classes stored first loads as it does
// alone (the previous head read the first four class files for the whole jar: "type Counter not found").
class C extends scala2traits.Counter
class P extends scala2traits.Named { def name = "p" }
class F extends scala2traits.Flag
@main def run() =
  val c = new C
  println(c.next)
  println(c.next)
  println(new P().greet)
  println(classOf[F].getDeclaredFields.filter(f => java.lang.reflect.Modifier.isVolatile(f.getModifiers)).map(_.getName).mkString(","))
