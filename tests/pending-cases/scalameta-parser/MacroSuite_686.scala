// Program 686 of tests/fix/scalameta, from scalameta 4.17.4's MacroSuite (line 171, "macro-quote-id-within-splice:
// ${ 'x }"): scalac 3.8.4's parser reads it as scalameta does, `${'{x}}`, and teq's otherwise. The expectation is
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
//               Term.SplicedMacroExpr 26 34
//                 Term.Block 28 34
//                   Term.QuotedMacroExpr 30 32
//                     Term.Name 31 32
object W686 {
  def w = {
$ { 'x }
}
}
