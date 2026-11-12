// Top-level inline vals of class constants, which dotty's Erasure drops with their getters: each read is the
// constant `BCodeBodyBuilder.genConstant` writes for its `ClazzTag`, a reference class by `ldc`, a primitive's or
// `Unit`'s boxed class's `TYPE`; read directly, from a lambda and through a constant final val's getter in another
// object (array classes: tests/cases/file_module_constants, on the JVM).
package classlits

class Meter(val value: Double) extends AnyVal

inline val str = classOf[String]
inline val int = classOf[Int]
inline val unit = classOf[Unit]
inline val meter = classOf[Meter]

object Holder:
  final val held = classOf[Boolean]

@main def run(): Unit =
  println(str)
  println(int)
  println(unit)
  println(meter.getSimpleName)
  val f = () => str.getSimpleName + " " + int
  println(f())
  println(Holder.held)
