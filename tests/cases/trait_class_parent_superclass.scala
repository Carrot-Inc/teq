// A class whose superclass comes from a library trait over a class: the class's superclass is the trait's class
// parent, as scalac's `Namer.ensureFirstIsClass` makes it, whether the trait comes first, through a trait of the
// program's, through a class, or after an interface. On the JVM such a class extended Object, so throwing or
// catching it failed verification.
import scala.util.control.NoStackTrace

class First extends NoStackTrace
trait Quiet extends NoStackTrace
class ThroughTrait extends Quiet
class Base extends NoStackTrace
class ThroughClass extends Base
class AfterInterface extends Runnable with NoStackTrace:
  def run(): Unit = ()
object Single extends NoStackTrace
case class Coded(code: Int) extends NoStackTrace
class Boxed[A](val value: A) extends NoStackTrace

def caught(t: => Throwable): String =
  try throw t
  catch
    case e: First => "First " + e.getStackTrace.length
    case e: ThroughTrait => "ThroughTrait " + e.getStackTrace.length
    case e: ThroughClass => "ThroughClass " + e.getStackTrace.length
    case e: AfterInterface => "AfterInterface " + e.getStackTrace.length
    case e: Coded => "Coded " + e.code
    case e: Boxed[?] => "Boxed " + e.value
    case e: NoStackTrace => "NoStackTrace " + (e eq Single)

@main def main(): Unit =
  for t <- List[() => Throwable](() => new First, () => new ThroughTrait, () => new ThroughClass, () => new AfterInterface, () => Single,
      () => Coded(3), () => Boxed("b"), () => new NoStackTrace {}) do
    println(caught(t()))
  val held: Throwable = new First
  println(held.getMessage)
