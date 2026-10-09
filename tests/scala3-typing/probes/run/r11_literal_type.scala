def speed(s: "fast" | "slow"): Int = s match
  case "fast" => 1
  case "slow" => 2
@main def run(): Unit =
  val x: 1 | 2 = 3
  println(x)
  println(speed("medium"))
