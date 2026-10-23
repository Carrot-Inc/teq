// Program 1060 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 2531, "#3261
// chained `match` with outdent, block, in try"): scalac 3.8.4's parser accepts it, teq's reports "expected end of
// statement, found 'match'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers
// to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 208
//     Defn.Object 0 207
//       Term.Name 7 12
//       Template 13 207
//         Template.Body 13 207
//           Defn.Object 15 204
//             Term.Name 22 27
//             Template 27 204
//               Template.Body 27 204
//                 Term.Try 31 204
//                   Term.Match 39 167
//                     Term.Block 39 109
//                       Term.Apply 39 44
//                         Term.Name 39 42
//                         Term.ArgClause 42 44
//                       Term.Match 49 109
//                         Term.Name 49 52
//                         Term.CasesBlock 63 109
//                           Case 63 82
//                             Term.Name 68 71
//                             Lit.String 75 82
//                           Case 87 109
//                             Pat.Wildcard 92 93
//                             Lit.String 99 109
//                     Term.CasesBlock 122 167
//                       Case 122 142
//                         Lit.String 127 134
//                         Lit.Int 141 142
//                       Case 147 167
//                         Lit.String 152 162
//                         Lit.Int 166 167
//                   Term.CasesBlock 180 204
//                     Case 180 204
//                       Pat.Var 185 186
//                         Term.Name 185 186
//                       Term.Apply 190 204
//                         Term.Select 190 202
//                           Term.Name 190 191
//                           Term.Name 192 202
//                         Term.ArgClause 202 204
object W1060 {
object small:
  try
    foo()
    Nil match
    case Nil => "empty"
    case _   => "nonempty"
  match
    case "empty"    => 0
    case "nonempty" => 1
  catch
    case e => e.getMessage()

}
