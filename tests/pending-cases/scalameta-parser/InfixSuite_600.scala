// Program 600 of tests/fix/scalameta, from scalameta 4.17.4's InfixSuite (line 747, "scalafmt #3825 2"): scalac
// 3.8.4's parser reads it as scalameta does, `module object a { module object b { foo.map(i => i + 1 + 2 + 3) *> bar
// baz } qux`, and teq's otherwise. The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree
// answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 131
//     Defn.Object 0 130
//       Term.Name 7 11
//       Template 12 130
//         Template.Body 12 130
//           Defn.Object 14 127
//             Term.Name 21 22
//             Template 22 127
//               Template.Body 22 127
//                 Defn.Object 26 121
//                   Term.Name 33 34
//                   Template 34 121
//                     Template.Body 34 121
//                       Term.ApplyInfix 40 113
//                         Term.Apply 40 100
//                           Term.Select 40 54
//                             Term.Name 40 43
//                             Term.Name 51 54
//                           Term.ArgClause 54 100
//                             Term.Block 54 100
//                               Term.Function 56 100
//                                 Term.ParamClause 56 57
//                                   Term.Param 56 57
//                                     Term.Name 56 57
//                                 Term.ApplyInfix 69 100
//                                   Term.ApplyInfix 69 88
//                                     Term.ApplyInfix 69 74
//                                       Term.Name 69 70
//                                       Term.Name 71 72
//                                       Type.ArgClause 73 73
//                                       Term.ArgClause 73 74
//                                         Lit.Int 73 74
//                                     Term.Name 85 86
//                                     Type.ArgClause 87 87
//                                     Term.ArgClause 87 88
//                                       Lit.Int 87 88
//                                   Term.Name 97 98
//                                   Type.ArgClause 99 99
//                                   Term.ArgClause 99 100
//                                     Lit.Int 99 100
//                         Term.Name 107 109
//                         Type.ArgClause 110 110
//                         Term.ArgClause 110 113
//                           Term.Name 110 113
//                       Term.Name 118 121
//                 Term.Name 124 127
object W600 {
object a:
  object b:
    foo
      .map: i =>
        i + 1
          + 2
        + 3
      *> bar
    baz
  qux

}
