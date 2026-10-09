// expect: type mismatch: found 3, required 1 | 2
// expect: type mismatch: found 3, required 3.14
// expect: type mismatch: found "medium", required "fast" | "slow"
def speed(s: "fast" | "slow"): Int = s match
  case "fast" => 1
  case "slow" => 2

@main def run(): Unit =
  val x: 1 | 2 = 3
  val pi: 3.14 = 3
  println(speed("medium"))
