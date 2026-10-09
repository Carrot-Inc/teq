// expect: 17:12: error: Recursion limit exceeded.
// expect: 1 error found
// Across an intersection an alias implements the other side's abstract member, as under
// scalac, so `A` and `B` are defined through each other and `z.A` never resolves: scalac's
// `Recursion limit exceeded` at the use.
object Test3 {
  trait W { type A; type B }
  trait X { z: W => type A = z.B; type B }
  trait Y { z: W => type A; type B = z.A }
  trait P { type C }
  trait Q { type C = Int }
  object App {
    type Z = X & Y
    val z: Z = z
    val q: P & Q = new P with Q {}
    val c: q.C = 1
    val a: z.A = 1
  }
}
