// Program 1110 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 268, "super.T"): scalac 3.8.4's
// parser accepts it, teq's reports "expected an identifier, found 'super'". The expectation is scalameta's tree
// under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 36
//     Defn.Object 0 35
//       Term.Name 7 12
//       Template 13 35
//         Template.Body 13 35
//           Defn.Type 17 33
//             Type.Name 22 23
//             Type.ParamClause 24 24
//             Type.Select 26 33
//               Term.Super 26 31
//                 Name.Anonymous 26 26
//                 Name.Anonymous 31 31
//               Type.Name 32 33
//             Type.Bounds 24 24
object W1110 {
  type T = super.T
}
