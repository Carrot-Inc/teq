// Program 341 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 594, "#3319 lambda simple
// lhs"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found identifier". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 66
//     Defn.Object 0 65
//       Term.Name 7 11
//       Template 12 65
//         Template.Body 12 65
//           Defn.Object 14 62
//             Term.Name 21 26
//             Template 26 62
//               Template.Body 26 62
//                 Term.ApplyInfix 30 62
//                   Term.Name 30 33
//                   Term.Name 34 41
//                   Type.ArgClause 41 41
//                   Term.ArgClause 41 62
//                     Term.Block 41 62
//                       Term.Function 43 62
//                         Term.ParamClause 43 44
//                           Term.Param 43 44
//                             Term.Name 43 44
//                         Term.Apply 52 62
//                           Term.Name 52 59
//                           Term.ArgClause 59 62
//                             Term.Name 60 61
object W341 {
object MyApp:
  ids foreach: x =>
    println(x)

}
