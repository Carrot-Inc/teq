// Two givens where the class is defined: the search for the implementation is ambiguous.
// expect: 8:9: error: ambiguous given instances for Int: a, b of inferring the implementation of the deferred given instance x in trait T
import scala.compiletime.deferred
trait T { given x: Int = deferred }
object Defs:
  given a: Int = 1
  given b: Int = 2
  class C extends T
