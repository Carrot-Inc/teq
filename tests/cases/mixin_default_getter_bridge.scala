// Generic traits whose getter forwarder is a bridge: one trait fixes the type argument and gives its own
// default, the winner keeps the type parameter, so the class forwards the fixed trait's erased getter to the
// winner's.
trait A[T] { def m(a: T = null.asInstanceOf[T]): String = "A" + a }
trait B extends A[String] { override def m(a: String = "b"): String = "B" + a }
trait D[T] extends A[T] { override def m(a: T = "d".asInstanceOf[T]): String = "D" + a }
class C extends B with D[String]

object Main:
  def main(args: Array[String]): Unit =
    println(new C().m())
    println((new C(): A[String]).m())
    println((new C(): B).m())
