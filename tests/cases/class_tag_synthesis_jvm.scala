//> using platform jvm
// A union or an intersection with `Unit` has no type symbol, so its `ClassTag` is that of its erasure's class,
// `Unit`'s box (`normalizeClass`), not `ClassTag.Unit`: the JVM's `scala.runtime.BoxedUnit`.
import scala.reflect.ClassTag
trait S
object Main:
  def t[T](using c: ClassTag[T]): String = s"$c ${c.runtimeClass.getName}"
  def main(args: Array[String]): Unit =
    println(t[Null | Unit])
    println(t[Unit & S])
