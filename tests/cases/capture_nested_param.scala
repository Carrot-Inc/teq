// A contextual function's parameter, named apart from the definition `contextual$1`, must also
// stay apart from the parameter of a def nested in it that reads the contextual parameter:
// `summon[Int]` is the given 1 and `contextual$1$1` the argument 2.
def `contextual$1`(): Int = 7

@main def main(): Unit =
  val g: Int ?=> Int =
    def k(`contextual$1$1`: Int) = summon[Int] + `contextual$1$1` + `contextual$1`()
    k(2)
  println(g(using 1))
