package b
import scala.annotation.nowarn
class P(override val v: String) extends a.P
@nowarn class Q extends a.A("a-arg") { override val v: String = "q-body" }
class A extends a.P { override val v: String = "b.A-body" }
