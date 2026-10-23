// Program 340 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 570, "#3319 lambda complex
// lhs"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found identifier". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 74
//     Defn.Object 0 73
//       Term.Name 7 11
//       Template 12 73
//         Template.Body 12 73
//           Defn.Object 14 70
//             Term.Name 21 26
//             Template 26 70
//               Template.Body 26 70
//                 Term.ApplyInfix 30 70
//                   Term.Apply 30 41
//                     Term.Name 30 34
//                     Term.ArgClause 34 41
//                       Lit.Int 35 36
//                       Lit.Int 37 38
//                       Lit.Int 39 40
//                   Term.Name 42 49
//                   Type.ArgClause 49 49
//                   Term.ArgClause 49 70
//                     Term.Block 49 70
//                       Term.Function 51 70
//                         Term.ParamClause 51 52
//                           Term.Param 51 52
//                             Term.Name 51 52
//                         Term.Apply 60 70
//                           Term.Name 60 67
//                           Term.ArgClause 67 70
//                             Term.Name 68 69
object W340 {
object MyApp:
  List(1,2,3) foreach: x =>
    println(x)

}
