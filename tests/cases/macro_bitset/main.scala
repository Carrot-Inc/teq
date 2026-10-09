import java.util.BitSet
import Macros.*

@main def run(): Unit =
  println(bits("aceg"))
  val bs = new BitSet(16)
  bs.set(1)
  bs.set(10, 12)
  bs.flip(0, 3)
  println(summary(bs))
  println(bs == bs.clone())
