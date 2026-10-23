// Program 699 of tests/fix/scalameta, from scalameta 4.17.4's MacroSuite (line 196, "macro-splice-id-within-quote:
// '{ $x }"): scalac 3.8.4's parser reads it as scalameta does, `'{`$x`}`, and teq's otherwise. The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 40
//     Defn.Object 0 39
//       Term.Name 7 11
//       Template 12 39
//         Template.Body 12 39
//           Defn.Def 16 37
//             Term.Name 20 21
//             Term.Block 24 37
//               Term.QuotedMacroExpr 26 35
//                 Term.Block 27 35
//                   Term.Name 29 33
object W699 {
  def w = {
'{ `$x` }
}
}
