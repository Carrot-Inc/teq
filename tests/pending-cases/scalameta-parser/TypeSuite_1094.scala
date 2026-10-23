// Program 1094 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 58, "with-type3"): scalac 3.8.4's
// parser accepts it, teq's reports "expected end of statement, found 'with'". The expectation is scalameta's tree
// under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 85
//     Defn.Object 0 84
//       Term.Name 7 12
//       Template 13 84
//         Template.Body 13 84
//           Defn.Type 15 81
//             Type.Name 20 21
//             Type.ParamClause 22 22
//             Type.Refine 24 81
//               Type.Name 24 31
//               Stat.Block 39 81
//                 Decl.Type 39 81
//                   Type.Name 44 45
//                   Type.ParamClause 45 45
//                   Type.Bounds 45 81
//                     Type.Refine 48 81
//                       Type.Name 48 52
//                       Stat.Block 64 81
//                         Decl.Type 64 81
//                           Type.Name 69 70
//                           Type.ParamClause 71 71
//                           Type.Bounds 71 81
//                             Type.Name 74 81
//             Type.Bounds 22 22
object W1094 {
type A = Product with
  type T>: Null
  with
    type D <: Product

}
