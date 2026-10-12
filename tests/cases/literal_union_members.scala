// A union of constants of one primitive class has that class's operators and conversions (its
// members are its join's, `OrType.join`): `u.toDouble`, `s + "x"`, `~n`.
@main def run() =
  val u: 1 | 2 = 1
  val s: "a" | "b" = "a"
  val n: -1 | 7 = 7
  println((u.toInt * 2, s + "x", ~n, u.toLong + 1L))
