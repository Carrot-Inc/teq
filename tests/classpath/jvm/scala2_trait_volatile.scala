// jars: scala-library scala2-traits
// A Scala 2.13 trait's `@volatile var` gives the field of the class mixing it in `ACC_VOLATILE`, as dotty's
// `Scala2Unpickler` reads the annotation of the var's field and `Mixin` copies it (master: a plain field, the
// transcoded pickle carrying no annotation); a var without it stays plain.
import java.lang.reflect.Modifier
class F extends scala2traits.Flag
@main def run() =
  val f = new F
  f.on = true
  println(f.on)
  def volatile(name: String) = Modifier.isVolatile(classOf[F].getDeclaredField(name).getModifiers)
  println(volatile("on"))
  println(volatile("plain"))
