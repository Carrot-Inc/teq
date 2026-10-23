package probeextra
import java.time.Instant
import java.math.BigDecimal
import java.lang.invoke.MethodHandle
package object nested:
  val n = 1
object Extra:
  val i = Instant.now()
  val b = BigDecimal.valueOf(1L)
  val d = BigDecimal.valueOf(1.0)
  val o = BigDecimal.ONE
  def mh(h: MethodHandle): Any = h.invokeExact(1)
  def locals: Int =
    val first = { val inner = 1; inner }
    val second = 2
    first + second
class Holder[F[_]]
object Higher:
  val a: Holder[[T] =>> List[T]] = ???
