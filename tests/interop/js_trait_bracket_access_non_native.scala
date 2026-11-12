// `@JSBracketAccess` on a member of a non-native JS trait is an error, and the call of it is to
// be a method's, not a bracket access with no index. The expectation is scalac 3.8.4's message
// under Scala.js 1.22.0, at the definition's start where scalac reports it at the member's name.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSBracketAccess
trait Bad extends js.Object:
  @JSBracketAccess def read(): Int
def use(b: Bad): Int = b.read()
