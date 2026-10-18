trait T[A]
trait U[A]
trait Holder { given old: Int }
object P {
 given named[A: T: U]: Ordering[A] = ???
 given impl: T[Int] with {}
 given plain: Int = 1
}
