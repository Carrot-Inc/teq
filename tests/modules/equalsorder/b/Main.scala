package equalsorder

object Main:
  def main(args: Array[String]): Unit =
    println(C(new X, 1) == C(new X, 2))
    println(C(new X, 1) == C(new X, 1))
    println(D(new X, "s", 1L, new X, true) == D(new X, "s", 1L, new X, false))
    println(D(new X, "s", 1L, new X, true) == D(new X, "t", 1L, new X, true))
