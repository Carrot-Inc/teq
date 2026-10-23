// Program 695 of tests/fix/scalameta, from scalameta 4.17.4's MacroSuite (line 191, "macro-splice-id-within-quote:
// '{ $x }"): scalac 3.8.4's parser accepts it, teq's reports "unterminated character literal". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 39
//     Defn.Object 0 38
//       Term.Name 7 11
//       Template 12 38
//         Template.Body 12 38
//           Defn.Def 16 36
//             Term.Name 20 21
//             Term.Block 24 36
//               Term.QuotedMacroExpr 26 34
//                 Term.Block 28 34
//                   Term.SplicedMacroExpr 30 32
//                     Term.Name 30 32
object W695 {
  def w = {
' { $x }
}
}
