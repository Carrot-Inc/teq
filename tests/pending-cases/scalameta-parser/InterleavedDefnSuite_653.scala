// Program 653 of tests/fix/scalameta, from scalameta 4.17.4's InterleavedDefnSuite (line 85, "def f[A](a: A, as:
// A*)[B]: B = ???"): scalac 3.8.4's parser accepts it, teq's reports "expected '=', found '['". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 51
//     Defn.Object 0 50
//       Term.Name 7 11
//       Template 12 50
//         Template.Body 12 50
//           Defn.Def 14 48
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
//             Member.ParamClauseGroup 36 39
//               Type.ParamClause 36 39
//                 Type.Param 37 38
//                   Type.Name 37 38
//                   Type.ParamClause 38 38
//                   Type.Bounds 38 38
//             Type.Name 41 42
//             Term.Name 45 48
object W653 {
def f[A](a: A, as: A*)[B]: B = ???
}
