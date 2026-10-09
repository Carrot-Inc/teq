// jars: fixtures
// A jar method's higher-kinded parameter keeps its lower bound, `F2[x] >: F[x]`: the given for
// the stream's own constructor is found, as scalac finds it, where another constructor's
// instance would conform without the bound.
import fix.hkb.HkStrm

object Main:
  def main(args: Array[String]): Unit =
    println(HkStrm[List, Int]().compile)
