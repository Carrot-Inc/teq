// Program 1057 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 2383, "#3261
// chained `match` with outdent, no block, in if cond"): scalac 3.8.4's parser accepts it, teq's reports "expected
// 'then', found 'match'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers
// to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 182
//     Defn.Object 0 181
//       Term.Name 7 12
//       Template 13 181
//         Template.Body 13 181
//           Defn.Object 15 178
//             Term.Name 22 27
//             Template 27 178
//               Template.Body 27 178
//                 Term.If 31 178
//                   Term.Match 38 163
//                     Term.Match 38 98
//                       Term.Name 38 41
//                       Term.CasesBlock 52 98
//                         Case 52 71
//                           Term.Name 57 60
//                           Lit.String 64 71
//                         Case 76 98
//                           Pat.Wildcard 81 82
//                           Lit.String 88 98
//                     Term.CasesBlock 111 163
//                       Case 111 134
//                         Lit.String 116 123
//                         Lit.Boolean 130 134
//                       Case 139 163
//                         Lit.String 144 154
//                         Lit.Boolean 158 163
//                   Term.Name 175 178
//                   Lit.Unit 179 179
object W1057 {
object small:
  if
    Nil match
    case Nil => "empty"
    case _   => "nonempty"
  match
    case "empty"    => true
    case "nonempty" => false
  then
    bar

}
