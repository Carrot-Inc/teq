// expect: 9:18: error: Found:    3⏎Required: 1 | 2
// expect: 10:18: error: Found:    3⏎Required: 3.14
// expect: 11:17: error: Found:    "medium"⏎Required: "fast" | "slow"
def speed(s: "fast" | "slow"): Int = s match
  case "fast" => 1
  case "slow" => 2

@main def run(): Unit =
  val x: 1 | 2 = 3
  val pi: 3.14 = 3
  println(speed("medium"))
