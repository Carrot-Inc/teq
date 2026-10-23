// Program 1099 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 211, "with-followed-by-brace"):
// scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found 'with'". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 98
//     Defn.Object 0 97
//       Term.Name 7 12
//       Template 13 97
//         Template.Body 13 97
//           Term.Block 15 94
//             Defn.Type 19 67
//               Type.Name 24 26
//               Type.ParamClause 27 27
//               Type.Refine 29 67
//                 Type.With 29 44
//                   Type.Name 29 35
//                   Type.Name 41 44
//                 Stat.Block 54 67
//                   Decl.Type 54 67
//                     Type.Name 59 60
//                     Type.ParamClause 60 60
//                     Type.Bounds 60 67
//                       Type.Name 63 67
//               Type.Bounds 27 27
//             Term.Block 70 92
//               Decl.Type 76 88
//                 Type.Name 81 82
//                 Type.ParamClause 82 82
//                 Type.Bounds 82 88
//                   Type.Name 85 88
object W1099 {
{
  type AA = String with Int with
    type T>: Null
  {
    type T>: Int
  }
}

}
