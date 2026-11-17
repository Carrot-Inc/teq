package lib
import scala.annotation.nowarn
class P { val v: String = "p"; def read: String = v }
class A(override val v: String) extends P
trait TR extends A
@nowarn class B extends A("a-arg") { override val v: String = "b-body" }
