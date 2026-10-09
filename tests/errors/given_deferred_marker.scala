// `scala.compiletime.deferred` is a marker of a given's right-hand side in a trait alone, and
// compile-time only anywhere else, as scalac's `@compileTimeOnly`.
// expect: 7:26: error: `deferred` can only be used as the right hand side of a given definition in a trait
// expect: 8:24: error: `deferred` can only be used as the right hand side of a given definition in a trait
// expect: 9:28: error: `deferred` can only be used as the right hand side of a given definition in a trait
import scala.compiletime.deferred
class C { given x: Int = deferred }
trait T { val y: Int = deferred }
object O { def z: String = scala.compiletime.deferred }
