// Program 1098 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 179,
// "coloneol-followed-by-brace-indent"): scalac 3.8.4's parser reads it as scalameta does, `type AA = (String & Int)
// { type T >: Null { type T >: Int } }`, and teq's otherwise. The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 85
//     Defn.Object 0 84
//       Term.Name 7 12
//       Template 13 84
//         Template.Body 13 84
//           Defn.Type 15 81
//             Type.Name 20 22
//             Type.ParamClause 23 23
//             Type.Refine 25 81
//               Type.With 25 40
//                 Type.Name 25 31
//                 Type.Name 37 40
//               Stat.Block 40 81
//                 Decl.Type 46 81
//                   Type.Name 51 52
//                   Type.ParamClause 52 52
//                   Type.Bounds 52 81
//                     Type.Refine 55 81
//                       Type.Name 55 59
//                       Stat.Block 59 81
//                         Decl.Type 69 81
//                           Type.Name 74 75
//                           Type.ParamClause 75 75
//                           Type.Bounds 75 81
//                             Type.Name 78 81
//             Type.Bounds 23 23
object W1098 {
type AA = String with Int:
    type T>: Null:
        type T>: Int

}
