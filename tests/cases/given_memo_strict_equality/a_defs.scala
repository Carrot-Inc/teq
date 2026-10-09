class A
class R(val n: Int)
trait Low { given low: R = new R(0) }
object R extends Low {
  given high(using CanEqual[A, A]): R = new R(1)
}
val warm = summon[R]
