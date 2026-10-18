// A search's target made of one type shared at every level (`S(n+1) = P[Sn, Sn]`): its classes are collected once
// a type (`Types.CoveringSetAccumulator`'s `seen` holds every type), so that the measure reads ten distinct
// applications where the target's tree holds 1,023 of them.
trait Mark
given Mark = new Mark {}
trait Show[A] { def name: String }
class P[A, B]
given show[A](using Mark): Show[A] = new Show[A] { def name = "show" }
type S0 = Int
type S1 = P[S0, S0]
type S2 = P[S1, S1]
type S3 = P[S2, S2]
type S4 = P[S3, S3]
type S5 = P[S4, S4]
type S6 = P[S5, S5]
type S7 = P[S6, S6]
type S8 = P[S7, S7]
type S9 = P[S8, S8]
type S10 = P[S9, S9]

@main def run(): Unit =
  println(summon[Show[S10]].name)
