// Program 1059 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 2481, "#3261
// chained `match` with outdent, no block, in try"): scalac 3.8.4's parser accepts it, teq's reports "expected end of
// statement, found 'match'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers
// to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 198
//     Defn.Object 0 197
//       Term.Name 7 12
//       Template 13 197
//         Template.Body 13 197
//           Defn.Object 15 194
//             Term.Name 22 27
//             Template 27 194
//               Template.Body 27 194
//                 Term.Try 31 194
//                   Term.Match 39 157
//                     Term.Match 39 99
//                       Term.Name 39 42
//                       Term.CasesBlock 53 99
//                         Case 53 72
//                           Term.Name 58 61
//                           Lit.String 65 72
//                         Case 77 99
//                           Pat.Wildcard 82 83
//                           Lit.String 89 99
//                     Term.CasesBlock 112 157
//                       Case 112 132
//                         Lit.String 117 124
//                         Lit.Int 131 132
//                       Case 137 157
//                         Lit.String 142 152
//                         Lit.Int 156 157
//                   Term.CasesBlock 170 194
//                     Case 170 194
//                       Pat.Var 175 176
//                         Term.Name 175 176
//                       Term.Apply 180 194
//                         Term.Select 180 192
//                           Term.Name 180 181
//                           Term.Name 182 192
//                         Term.ArgClause 192 194
object W1059 {
object small:
  try
    Nil match
    case Nil => "empty"
    case _   => "nonempty"
  match
    case "empty"    => 0
    case "nonempty" => 1
  catch
    case e => e.getMessage()

}
