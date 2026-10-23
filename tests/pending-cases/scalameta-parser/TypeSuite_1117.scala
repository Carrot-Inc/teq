// Program 1117 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 286, "A with B {}"): scalac 3.8.4's
// parser accepts it, teq's reports "expected end of statement, found '{'". The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 40
//     Defn.Object 0 39
//       Term.Name 7 12
//       Template 13 39
//         Template.Body 13 39
//           Defn.Type 17 37
//             Type.Name 22 23
//             Type.ParamClause 24 24
//             Type.Refine 26 37
//               Type.With 26 34
//                 Type.Name 26 27
//                 Type.Name 33 34
//               Stat.Block 35 37
//             Type.Bounds 24 24
object W1117 {
  type T = A with B {}
}
