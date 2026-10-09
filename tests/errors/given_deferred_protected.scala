// The implementation of a protected deferred given is protected, as dotty's `dcl.copy` keeps the
// declaration's flags: reading it from outside is refused, and so over the products, whose pickle
// writes it `PROTECTED` (scalac's E173 there too). The message names the class holding the
// implementation, as scalac's does and as a build over the products does.
// expect: 9:33: error: given x in class C cannot be accessed as a member of C from here; protected given x can only be accessed from class C or one of its subclasses
import scala.compiletime.deferred
trait T { protected given x: Int = deferred }
class C(using Int) extends T
@main def run(): Unit = println(new C(using 12).x)
