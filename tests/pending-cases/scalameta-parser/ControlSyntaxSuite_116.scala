// Program 116 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 2049, "match-chained-
// complex"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found identifier". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 158
//     Defn.Object 0 157
//       Term.Name 7 11
//       Template 12 157
//         Template.Body 12 157
//           Defn.Val 14 154
//             Pat.Var 18 23
//               Term.Name 18 23
//             Term.Match 26 154
//               Term.ApplyInfix 26 109
//                 Term.Match 26 90
//                   Term.Name 26 28
//                   Term.CasesBlock 35 90
//                     Case 39 58
//                       Term.Name 44 47
//                       Lit.String 51 58
//                     Case 61 88
//                       Pat.ExtractInfix 66 74
//                         Pat.Var 66 67
//                           Term.Name 66 67
//                         Term.Name 68 70
//                         Pat.ArgClause 71 74
//                           Pat.Var 71 74
//                             Term.Name 71 74
//                       Lit.String 78 88
//                 Term.Name 91 101
//                 Type.ArgClause 102 102
//                 Term.ArgClause 102 109
//                   Lit.String 102 109
//               Term.CasesBlock 116 154
//                 Case 120 134
//                   Lit.Boolean 125 129
//                   Lit.Int 133 134
//                 Case 137 152
//                   Lit.Boolean 142 147
//                   Lit.Int 151 152
object W116 {
val hello = xs match {
  case Nil => "empty"
  case x :: xs1 => "nonempty"
} startsWith "empty" match {
  case true => 0
  case false => 1
}

}
