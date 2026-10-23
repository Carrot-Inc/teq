// Program 1097 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 144, "with-followed-by-brace-
// indent"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found 'with'". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 104
//     Defn.Object 0 103
//       Term.Name 7 12
//       Template 13 103
//         Template.Body 13 103
//           Defn.Type 15 100
//             Type.Name 20 22
//             Type.ParamClause 23 23
//             Type.Refine 25 100
//               Type.With 25 40
//                 Type.Name 25 31
//                 Type.Name 37 40
//               Stat.Block 50 100
//                 Decl.Type 50 100
//                   Type.Name 55 56
//                   Type.ParamClause 56 56
//                   Type.Bounds 56 100
//                     Type.Refine 59 100
//                       Type.Name 59 63
//                       Stat.Block 70 100
//                         Decl.Type 80 92
//                           Type.Name 85 86
//                           Type.ParamClause 86 86
//                           Type.Bounds 86 92
//                             Type.Name 89 92
//             Type.Bounds 23 23
object W1097 {
type AA = String with Int with
    type T>: Null
      {
        type T>: Int
      }

}
