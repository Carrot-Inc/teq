// Program 120 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 2208, "match-chained-complex-
// operator 4"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found identifier". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 131
//     Defn.Object 0 130
//       Term.Name 7 11
//       Template 12 130
//         Template.Body 12 130
//           Defn.Val 14 126
//             Pat.Var 18 23
//               Term.Name 18 23
//             Term.Match 26 126
//               Term.ApplyInfix 26 85
//                 Term.Match 26 77
//                   Term.ApplyInfix 26 34
//                     Lit.Int 26 27
//                     Term.Name 28 31
//                     Type.ArgClause 32 32
//                     Term.ArgClause 32 34
//                       Term.Name 32 34
//                   Term.CasesBlock 43 77
//                     Case 43 56
//                       Term.Name 48 51
//                       Lit.Int 55 56
//                     Case 59 77
//                       Pat.ExtractInfix 64 72
//                         Pat.Var 64 65
//                           Term.Name 64 65
//                         Term.Name 66 68
//                         Pat.ArgClause 69 72
//                           Pat.Var 69 72
//                             Term.Name 69 72
//                       Lit.Int 76 77
//                 Term.Name 78 83
//                 Type.ArgClause 84 84
//                 Term.ArgClause 84 85
//                   Lit.Int 84 85
//               Term.CasesBlock 94 126
//                 Case 94 108
//                   Lit.Int 99 100
//                   Lit.Boolean 104 108
//                 Case 111 126
//                   Lit.Int 116 117
//                   Lit.Boolean 121 126
object W120 {
val hello = 1 foo xs match
  case Nil => 0
  case x :: xs1 => 1
`foo` 1 match
  case 1 => true
  case 2 => false


}
