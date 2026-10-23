// Program 643 of tests/fix/scalameta, from scalameta 4.17.4's InterleavedDeclSuite (line 87, "f[A](a: A, as:
// A*)[B](b: B, bs: B*)[C](implicit c: C): B [def with breaks]"): scalac 3.8.4's parser accepts it, teq's reports
// "expected an expression, found '['". The expectation is scalameta's tree under dialects.Scala3, which teq fix's
// tree answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 99
//     Defn.Object 0 98
//       Term.Name 7 11
//       Template 12 98
//         Template.Body 12 98
//           Decl.Def 14 95
//             Term.Name 18 19
//             Member.ParamClauseGroup 22 42
//               Type.ParamClause 22 25
//                 Type.Param 23 24
//                   Type.Name 23 24
//                   Type.ParamClause 24 24
//                   Type.Bounds 24 24
//               Term.ParamClause 28 42
//                 Term.Param 29 33
//                   Term.Name 29 30
//                   Type.Name 32 33
//                 Term.Param 35 41
//                   Term.Name 35 37
//                   Type.Repeated 39 41
//                     Type.Name 39 40
//             Member.ParamClauseGroup 45 65
//               Type.ParamClause 45 48
//                 Type.Param 46 47
//                   Type.Name 46 47
//                   Type.ParamClause 47 47
//                   Type.Bounds 47 47
//               Term.ParamClause 51 65
//                 Term.Param 52 56
//                   Term.Name 52 53
//                   Type.Name 55 56
//                 Term.Param 58 64
//                   Term.Name 58 60
//                   Type.Repeated 62 64
//                     Type.Name 62 63
//             Member.ParamClauseGroup 68 89
//               Type.ParamClause 68 71
//                 Type.Param 69 70
//                   Type.Name 69 70
//                   Type.ParamClause 70 70
//                   Type.Bounds 70 70
//               Term.ParamClause 74 89
//                 Term.Param 84 88
//                   Mod.Implicit 75 83
//                   Term.Name 84 85
//                   Type.Name 87 88
//                 Mod.Implicit 75 83
//             Type.Name 94 95
object W643 {
def f
  [A]
  (a: A, as: A*)
  [B]
  (b: B, bs: B*)
  [C]
  (implicit c: C)
  : B

}
