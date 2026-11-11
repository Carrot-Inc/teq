// `ordinal` of a Java enum's constant called through `java.lang.Enum[?]`: scalac 3.8.4's program prints 4, 4 and 0;
// teq's JavaScript run prints `undefined` for the two calls through `Enum[?]` (the direct `TimeUnit.MINUTES.ordinal`
// gives 4); its JVM target and `teq interp` print 4, 4 and 0. Met by the closure adversary of the pass on 5bd85bcd1
// (round 23) outside its closure.
def h(x: java.lang.Enum[?]): Int = x.ordinal
@main def run(): Unit =
  println(java.util.concurrent.TimeUnit.MINUTES.ordinal)
  println(h(java.util.concurrent.TimeUnit.MINUTES))
  println(h(java.time.DayOfWeek.MONDAY))
