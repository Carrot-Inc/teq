// Program 1092 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 17, "with-type"): scalac 3.8.4's
// parser accepts it, teq's reports "expected end of statement, found 'with'". The expectation is scalameta's tree
// under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 55
//     Defn.Object 0 54
//       Term.Name 7 12
//       Template 13 54
//         Template.Body 13 54
//           Defn.Type 15 51
//             Type.Name 20 21
//             Type.ParamClause 22 22
//             Type.Refine 24 51
//               Type.Name 24 30
//               Stat.Block 38 51
//                 Decl.Type 38 51
//                   Type.Name 43 44
//                   Type.ParamClause 44 44
//                   Type.Bounds 44 51
//                     Type.Name 47 51
//             Type.Bounds 22 22
object W1092 {
type A = AnyRef with
  type T>: Null

}
