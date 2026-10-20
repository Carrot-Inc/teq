// lila: modules/analyse/src/main/AccuracyPercent.scala:136 (`byPhase.map((phase, acc) => phase -> acc(color)).toMap`)
// `phase` is a `GamePhase`, a declared union of literal types: `phase -> n` keeps it (dotty widens a soft union
// and a singleton, not a declared union), so `toMap` finds `(GamePhase, Int) <:< (GamePhase, Int)`.
type GamePhase = "opening" | "middlegame" | "endgame"

@main def run(): Unit =
  val byPhase: List[(GamePhase, Int)] = List("opening" -> 1, "endgame" -> 3)
  val m: Map[GamePhase, Int] = byPhase.map((phase, n) => phase -> n).toMap
  println(m)
