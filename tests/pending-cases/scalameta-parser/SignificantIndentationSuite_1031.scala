// Program 1031 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 1571, "then-same-
// line"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found 'then'". The expectation
// is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 125
//     Defn.Object 0 124
//       Term.Name 7 12
//       Template 13 124
//         Template.Body 13 124
//           Defn.Def 15 121
//             Term.Name 19 20
//             Term.If 26 121
//               Term.Apply 35 67
//                 Term.Select 35 43
//                   Term.Name 35 36
//                   Term.Name 37 43
//                 Term.ArgClause 53 67
//                   Term.Function 54 66
//                     Term.ParamClause 54 55
//                       Term.Param 54 55
//                         Term.Name 54 55
//                     Term.ApplyInfix 59 66
//                       Term.Name 59 60
//                       Term.Name 61 63
//                       Type.ArgClause 64 64
//                       Term.ArgClause 64 66
//                         Lit.Int 64 66
//               Term.Apply 79 93
//                 Term.Name 79 86
//                 Term.ArgClause 86 93
//                   Lit.String 87 92
//               Term.Apply 108 121
//                 Term.Name 108 115
//                 Term.ArgClause 115 121
//                   Lit.String 116 120
object W1031 {
def f =
   if
      x.exists
         (x => x == 10) then
      println("Yes")
   else
      println("No")

}
