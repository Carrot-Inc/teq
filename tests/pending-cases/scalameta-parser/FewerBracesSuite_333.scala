// Program 333 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 325, "chain"): scalac 3.8.4's
// parser reads it as scalameta does, `val a: Int = xs.map(x => x * x).filter((y: Int) => y > 0)(0)`, and teq's
// otherwise. The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a
// line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 105
//     Defn.Object 0 104
//       Term.Name 7 11
//       Template 12 104
//         Template.Body 12 104
//           Defn.Val 14 101
//             Pat.Var 18 19
//               Term.Name 18 19
//             Type.Name 21 24
//             Term.Apply 27 101
//               Term.Apply 27 93
//                 Term.Select 27 68
//                   Term.Apply 27 56
//                     Term.Select 27 38
//                       Term.Name 27 29
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
//                               Term.Name 55 56
//                   Term.Name 62 68
//                 Term.ArgClause 68 93
//                   Term.Block 68 93
//                     Term.Function 70 93
//                       Term.ParamClause 70 78
//                         Term.Param 71 77
//                           Term.Name 71 72
//                           Type.Name 74 77
//                       Term.ApplyInfix 88 93
//                         Term.Name 88 89
//                         Term.Name 90 91
//                         Type.ArgClause 92 92
//                         Term.ArgClause 92 93
//                           Lit.Int 92 93
//               Term.ArgClause 98 101
//                 Lit.Int 99 100
object W333 {
val a: Int = xs
    .map: x =>
      x * x
    .filter: (y: Int) =>
      y > 0
    (0)

}
