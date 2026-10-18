class Pair[A, B]
infix class IPair[A, B]
type Al[A, B] = Pair[A, B]
infix type IAl[A, B] = Pair[A, B]
object P {
  type T1 = Int Either String
  type T2 = Int Pair String
  type T3 = Int IPair String
  type T4 = Int Al String
  type T5 = Int IAl String
  type T6 = Int `Pair` String
  type T7 = Int Map String
  type T8 = Pair[Int, String]
}
