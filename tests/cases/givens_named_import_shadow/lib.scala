//> using scala 3.8.4
package lib
trait T { def n: String }
trait U { def n: String }
object A:
  given g: T with { def n = "A.g" }
  given u: U with { def n = "A.u" }
object B:
  given g: T with { def n = "B.g" }
  given u: U with { def n = "B.u" }
object LocalU extends U { def n = "local u" }
