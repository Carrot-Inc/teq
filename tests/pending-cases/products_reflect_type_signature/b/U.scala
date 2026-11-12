// A downstream of a/ as tests/modules builds it
// (tests/modules/macrousingoverload's readtasty-a). scalac
// 3.8.4 prints the .expected. teq's pickle of a/ is to
// sign `Pc.apply(using q: Quotes)(t: q.reflect.Term)` as
// scalac does, with the abstract type member's erasure,
// `(scala.quoted.Quotes, java.lang.Object)`, so that scalac's reader
// finds the member. A reflection trait's parameter likewise (scalac signs
// `scala.quoted.Quotes.reflectModule.Printer`, tests/modules/macroclausekinds' readtasty-a).
package rtb

@main def run(): Unit =
  println(rts.Pc(7).n)
