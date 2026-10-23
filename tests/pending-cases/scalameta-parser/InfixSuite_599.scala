// Program 599 of tests/fix/scalameta, from scalameta 4.17.4's InfixSuite (line 696, "scalafmt #3825 1"): scalac
// 3.8.4's parser reads it as scalameta does, `module object a { foo.map(i => i + 1) *> bar`, and teq's otherwise.
// The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its
// start and end in UTF-16 units from the program's first line below.
//   Source 0 71
//     Defn.Object 0 70
//       Term.Name 7 11
//       Template 12 70
//         Template.Body 12 70
//           Defn.Object 14 67
//             Term.Name 21 22
//             Template 22 67
//               Template.Body 22 67
//                 Term.ApplyInfix 26 67
//                   Term.Apply 26 56
//                     Term.Select 26 38
//                       Term.Name 26 29
//                       Term.Name 35 38
//                     Term.ArgClause 38 56
//                       Term.Block 38 56
//                         Term.Function 40 56
//                           Term.ParamClause 40 41
//                             Term.Param 40 41
//                               Term.Name 40 41
//                           Term.ApplyInfix 51 56
//                             Term.Name 51 52
//                             Term.Name 53 54
//                             Type.ArgClause 55 55
//                             Term.ArgClause 55 56
//                               Lit.Int 55 56
//                   Term.Name 61 63
//                   Type.ArgClause 64 64
//                   Term.ArgClause 64 67
//                     Term.Name 64 67
object W599 {
object a:
  foo
    .map: i =>
      i + 1
    *> bar

}
