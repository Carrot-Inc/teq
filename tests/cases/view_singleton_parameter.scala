// ImplicitRefs.adjustSingletonArg widens a conversion's parameters with ImplicitRefs' own widenSingleton
// (Implicits.scala:113-118, 214): a parameter deriving from scala.Singleton is Any, as methodCandidateKind's
// test took it, so the conversion stays eligible for a literal argument.
import scala.language.implicitConversions

final class Wrapped(val value: Int)
object Wrapped:
  implicit def fromSingleton(x: Singleton): Wrapped = new Wrapped(1)

object Use:
  val x: Wrapped = 1

@main def run(): Unit =
  println(Use.x.value)
