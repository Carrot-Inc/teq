// A method of the deferred given's name with other parameters overloads it, implementing nothing
// (scalac synthesizes beside it): teq's "a val beside a method of its name is an alternative in a
// class only", the implementation being no member of the class's table.
// expect: 7:7: error: given x of type Int does not match method x in class C of type (s: String): String; only methods can be overloaded
import scala.compiletime.deferred
trait T { given x: Int = deferred }
class C(using Int) extends T { def x(s: String): String = s }
