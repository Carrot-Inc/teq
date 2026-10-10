import scala.language.implicitConversions
class A
class B(val n: Int)
class Ev(val n: Int)
trait LowPrio { given fn: (A => B) = a => new B(0) }
object Conv extends LowPrio { implicit def conv(a: A)(using ev: Ev): B = new B(ev.n) }
object Ev { given ev(using f: A => B): Ev = new Ev(1) }
import Conv.{*, given}

val warm = summon[A => B]
@main def main(): Unit = println(summon[A => B](new A).n)
