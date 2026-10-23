// Program 1055 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 2290, "#3261
// chained `match` with outdent, no block, in assign"): scalac 3.8.4's parser accepts it, teq's reports "expected end
// of statement, found 'match'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree
// answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 169
//     Defn.Object 0 168
//       Term.Name 7 12
//       Template 13 168
//         Template.Body 13 168
//           Defn.Object 15 165
//             Term.Name 22 27
//             Template 27 165
//               Template.Body 27 165
//                 Defn.Val 31 165
//                   Pat.Var 35 40
//                     Term.Name 35 40
//                   Term.Match 47 165
//                     Term.Match 47 107
//                       Term.Name 47 50
//                       Term.CasesBlock 61 107
//                         Case 61 80
//                           Term.Name 66 69
//                           Lit.String 73 80
//                         Case 85 107
//                           Pat.Wildcard 90 91
//                           Lit.String 97 107
//                     Term.CasesBlock 120 165
//                       Case 120 140
//                         Lit.String 125 132
//                         Lit.Int 139 140
//                       Case 145 165
//                         Lit.String 150 160
//                         Lit.Int 164 165
object W1055 {
object small:
  val value =
    Nil match
    case Nil => "empty"
    case _   => "nonempty"
  match
    case "empty"    => 0
    case "nonempty" => 1

}
