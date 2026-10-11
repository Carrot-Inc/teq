// jars: scala-library scala2-traits-mixed
// Each class of a jar is Scala 2's or not by its own TASTy or class file, as dotty's `ClassfileParser` dispatches on
// each class (`unpickleOrParseInnerClasses`): the scala2-traits-padded jar with a Scala 3 class and its TASTy added
// loads its Scala 2 traits from their pickles and the Scala 3 class from its TASTy (the previous head took a jar
// holding any TASTy for no Scala 2 one: "type Counter not found").
class C extends scala2traits.Counter
class P extends scala2traits.Named { def name = "p" }
class F extends scala2traits.Flag
@main def run() =
  val c = new C
  println(c.next)
  println(c.next)
  println(new P().greet)
  println(classOf[F].getDeclaredFields.filter(f => java.lang.reflect.Modifier.isVolatile(f.getModifiers)).map(_.getName).mkString(","))
  println(new unrelated.Other().answer)
