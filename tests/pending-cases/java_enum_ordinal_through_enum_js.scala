// `ordinal` of a Java enum's constant called
// through `java.lang.Enum[?]`, beside the direct
// `TimeUnit.MINUTES.ordinal`: scalac 3.8.4's program prints
// 4, 4 and 0, and teq's JavaScript run is to print them too.
def h(x: java.lang.Enum[?]): Int = x.ordinal
@main def run(): Unit =
  println(java.util.concurrent.TimeUnit.MINUTES.ordinal)
  println(h(java.util.concurrent.TimeUnit.MINUTES))
  println(h(java.time.DayOfWeek.MONDAY))
