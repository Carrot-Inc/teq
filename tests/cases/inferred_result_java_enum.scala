//> using platform jvm
// std: scala-library
// A final val of a Java enum's value keeps the value's singleton type,
// overriding a member or not (dotty's Namer.inferredResultType,
// `isJavaEnumValue`). The JDK's ElementType is read from its class file.
import java.lang.annotation.ElementType
trait E:
  def d: ElementType
class EU extends E:
  final val d = ElementType.FIELD
  final val own = ElementType.METHOD
@main def run(): Unit =
  val e = new EU
  val m: ElementType.FIELD.type = e.d
  val f: ElementType.METHOD.type = e.own
  println(m.toString + " " + f)
