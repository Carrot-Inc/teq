// Two traits' deferred givens of one name share the one implementation, which has to implement each:
// one of another type is scalac's E164 at the class, against the most derived one's type, where the
// postponed conflict check let a String implementation stand for an Int declaration.
// expect: 9:7: error: error overriding given x in trait B of type Int: given x of type String has incompatible type
import scala.compiletime.deferred
trait A { given x: Int = deferred }
trait B { given x: Int = deferred }
trait D { given x: String = deferred }
class C(using Int, String) extends A with B with D
