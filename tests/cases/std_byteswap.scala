// scala.util.hashing's byteswap32 and byteswap64, which boopickle's identity maps hash with.
import scala.util.hashing.{byteswap32, byteswap64}
object Main:
  def main(args: Array[String]): Unit =
    println(List(0, 1, -1, 42, Int.MaxValue, 123456789).map(byteswap32))
    println(List(0L, 1L, -1L, 42L, Long.MaxValue, 1234567890123L).map(byteswap64))
