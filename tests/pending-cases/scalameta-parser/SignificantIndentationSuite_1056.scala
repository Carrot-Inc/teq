// Program 1056 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 2336, "#3261
// chained `match` with outdent, block, in assign"): scalac 3.8.4's parser accepts it, teq's reports "expected end of
// statement, found 'match'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers
// to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 179
//     Defn.Object 0 178
//       Term.Name 7 12
//       Template 13 178
//         Template.Body 13 178
//           Defn.Object 15 175
//             Term.Name 22 27
//             Template 27 175
//               Template.Body 27 175
//                 Defn.Val 31 175
//                   Pat.Var 35 40
//                     Term.Name 35 40
//                   Term.Match 47 175
//                     Term.Block 47 117
//                       Term.Apply 47 52
//                         Term.Name 47 50
//                         Term.ArgClause 50 52
//                       Term.Match 57 117
//                         Term.Name 57 60
//                         Term.CasesBlock 71 117
//                           Case 71 90
//                             Term.Name 76 79
//                             Lit.String 83 90
//                           Case 95 117
//                             Pat.Wildcard 100 101
//                             Lit.String 107 117
//                     Term.CasesBlock 130 175
//                       Case 130 150
//                         Lit.String 135 142
//                         Lit.Int 149 150
//                       Case 155 175
//                         Lit.String 160 170
//                         Lit.Int 174 175
object W1056 {
object small:
  val value =
    foo()
    Nil match
    case Nil => "empty"
    case _   => "nonempty"
  match
    case "empty"    => 0
    case "nonempty" => 1

}
