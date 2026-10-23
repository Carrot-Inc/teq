// Program 309 of tests/fix/scalameta, from scalameta 4.17.4's ExtensionMethodsSuite (line 206, "extension-named-
// method"): scalac 3.8.4's parser accepts it, teq's reports "expected an identifier, found literal". The expectation
// is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 92
//     Defn.Object 0 91
//       Term.Name 7 11
//       Template 12 91
//         Template.Body 12 91
//           Defn.Def 16 89
//             Term.Name 20 21
//             Term.Block 24 89
//               Defn.Object 26 87
//                 Term.Name 33 34
//                 Template 35 87
//                   Template.Body 35 87
//                     Defn.Def 39 68
//                       Term.Name 43 52
//                       Member.ParamClauseGroup 52 60
//                         Type.ParamClause 52 52
//                         Term.ParamClause 52 60
//                           Term.Param 53 59
//                             Term.Name 53 54
//                             Type.Name 56 59
//                       Term.ApplyInfix 63 68
//                         Term.Name 63 64
//                         Term.Name 65 66
//                         Type.ArgClause 67 67
//                         Term.ArgClause 67 68
//                           Lit.Int 67 68
//                     Term.Apply 71 85
//                       Term.Name 71 82
//                       Term.ArgClause 82 85
//                         Lit.Int 83 84
object W309 {
  def w = {
object A {
  def extension(a: Int) = a + 2
  `extension`(2)
}
}
}
