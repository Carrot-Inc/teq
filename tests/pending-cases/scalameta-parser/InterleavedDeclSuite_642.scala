// Program 642 of tests/fix/scalameta, from scalameta 4.17.4's InterleavedDeclSuite (line 65, "def f[A](a: A, as:
// A*)[B](b: B, bs: B*)[C](implicit c: C): B"): scalac 3.8.4's parser accepts it, teq's reports "expected '=', found
// '['". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line,
// its start and end in UTF-16 units from the program's first line below.
//   Source 0 77
//     Defn.Object 0 76
//       Term.Name 7 11
//       Template 12 76
//         Template.Body 12 76
//           Decl.Def 14 74
//             Term.Name 18 19
//             Member.ParamClauseGroup 19 36
//               Type.ParamClause 19 22
//                 Type.Param 20 21
//                   Type.Name 20 21
//                   Type.ParamClause 21 21
//                   Type.Bounds 21 21
//               Term.ParamClause 22 36
//                 Term.Param 23 27
//                   Term.Name 23 24
//                   Type.Name 26 27
//                 Term.Param 29 35
//                   Term.Name 29 31
//                   Type.Repeated 33 35
//                     Type.Name 33 34
//             Member.ParamClauseGroup 36 53
//               Type.ParamClause 36 39
//                 Type.Param 37 38
//                   Type.Name 37 38
//                   Type.ParamClause 38 38
//                   Type.Bounds 38 38
//               Term.ParamClause 39 53
//                 Term.Param 40 44
//                   Term.Name 40 41
//                   Type.Name 43 44
//                 Term.Param 46 52
//                   Term.Name 46 48
//                   Type.Repeated 50 52
//                     Type.Name 50 51
//             Member.ParamClauseGroup 53 71
//               Type.ParamClause 53 56
//                 Type.Param 54 55
//                   Type.Name 54 55
//                   Type.ParamClause 55 55
//                   Type.Bounds 55 55
//               Term.ParamClause 56 71
//                 Term.Param 66 70
//                   Mod.Implicit 57 65
//                   Term.Name 66 67
//                   Type.Name 69 70
//                 Mod.Implicit 57 65
//             Type.Name 73 74
object W642 {
def f[A](a: A, as: A*)[B](b: B, bs: B*)[C](implicit c: C): B
}
