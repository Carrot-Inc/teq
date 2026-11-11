// `@JSBracketAccess` on a member of a non-native JS trait is the preparation's error, reported once the typing is
// joined; the call typed before it is a method's, not a bracket access with no index. scalac 3.8.4 under Scala.js
// 1.22.0 reports the same message, at the member's name where teq reports its definition's start.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSBracketAccess
trait Bad extends js.Object:
  @JSBracketAccess def read(): Int
def use(b: Bad): Int = b.read()
