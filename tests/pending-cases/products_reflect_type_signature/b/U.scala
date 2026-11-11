// A downstream of a/ as tests/modules builds it (tests/modules/macrousingoverload's readtasty-a). scalac 3.8.4 prints
// the .expected. teq's pickle of a/ signs `Pc.apply(using q: Quotes)(t: q.reflect.Term)` with the std's class,
// `(scala.quoted.Quotes, scala.quoted.Reflect$.Term)`, where scalac signs the abstract type member's erasure,
// `(scala.quoted.Quotes, java.lang.Object)`, so scalac's reader finds no member of the signature ("undefined:
// muo.Pc.apply"). The writer's member signatures name teq's std's reflection classes (`sig_class_name`), which the
// reflection table's keys rely on; a library's signature reads teq's and scalac's alike (push 15). Found beside
// macrousingoverload; a reflection trait's parameter likewise (`scala.quoted.Reflect$.Printer` where scalac signs
// `scala.quoted.Quotes.reflectModule.Printer`, tests/modules/macroclausekinds' readtasty-a).
package rtb

@main def run(): Unit =
  println(rts.Pc(7).n)
