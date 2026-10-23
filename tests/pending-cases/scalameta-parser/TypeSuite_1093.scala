// Program 1093 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 36, "with-type2"): scalac 3.8.4's
// parser accepts it, teq's reports "expected end of statement, found 'with'". The expectation is scalameta's tree
// under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 68
//     Defn.Object 0 67
//       Term.Name 7 12
//       Template 13 67
//         Template.Body 13 67
//           Defn.Type 15 64
//             Type.Name 20 21
//             Type.ParamClause 22 22
//             Type.Refine 24 64
//               Type.With 24 43
//                 Type.Name 24 30
//                 Type.Name 36 43
//               Stat.Block 51 64
//                 Decl.Type 51 64
//                   Type.Name 56 57
//                   Type.ParamClause 57 57
//                   Type.Bounds 57 64
//                     Type.Name 60 64
//             Type.Bounds 22 22
object W1093 {
type A = AnyRef with Product with
  type T>: Null

}
