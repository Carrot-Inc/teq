@main def main(): Unit =
  println(Probe.cond(if 1 > 0 then "a" else "b"))
  println(Probe.fun(math.max(1, 2)))
  println(Probe.fun(42))
