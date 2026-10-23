// Program 1058 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 2431, "#3261
// chained `match` with outdent, block, in if cond"): scalac 3.8.4's parser accepts it, teq's reports "expected
// 'then', found 'match'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers
// to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 192
//     Defn.Object 0 191
//       Term.Name 7 12
//       Template 13 191
//         Template.Body 13 191
//           Defn.Object 15 188
//             Term.Name 22 27
//             Template 27 188
//               Template.Body 27 188
//                 Term.If 31 188
//                   Term.Match 38 173
//                     Term.Block 38 108
//                       Term.Apply 38 43
//                         Term.Name 38 41
//                         Term.ArgClause 41 43
//                       Term.Match 48 108
//                         Term.Name 48 51
//                         Term.CasesBlock 62 108
//                           Case 62 81
//                             Term.Name 67 70
//                             Lit.String 74 81
//                           Case 86 108
//                             Pat.Wildcard 91 92
//                             Lit.String 98 108
//                     Term.CasesBlock 121 173
//                       Case 121 144
//                         Lit.String 126 133
//                         Lit.Boolean 140 144
//                       Case 149 173
//                         Lit.String 154 164
//                         Lit.Boolean 168 173
//                   Term.Name 185 188
//                   Lit.Unit 189 189
object W1058 {
object small:
  if
    foo()
    Nil match
    case Nil => "empty"
    case _   => "nonempty"
  match
    case "empty"    => true
    case "nonempty" => false
  then
    bar

}
