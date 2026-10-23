// Program 343 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 650, "#3319 lambda block"):
// scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found identifier". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 73
//     Defn.Object 0 72
//       Term.Name 7 11
//       Template 12 72
//         Template.Body 12 72
//           Defn.Object 14 69
//             Term.Name 21 26
//             Template 26 69
//               Template.Body 26 69
//                 Term.ApplyInfix 30 69
//                   Term.Name 30 33
//                   Term.Name 34 37
//                   Type.ArgClause 37 37
//                   Term.ArgClause 37 69
//                     Term.Block 37 69
//                       Term.Function 39 69
//                         Term.ParamClause 39 40
//                           Term.Param 39 40
//                             Term.Name 39 40
//                         Term.Block 50 69
//                           Term.Apply 50 56
//                             Term.Name 50 53
//                             Term.ArgClause 53 56
//                               Term.Name 54 55
//                           Term.Apply 63 69
//                             Term.Name 63 66
//                             Term.ArgClause 66 69
//                               Term.Name 67 68
object W343 {
object MyApp:
  ids map: x =>
      foo(x)
      bar(x)

}
