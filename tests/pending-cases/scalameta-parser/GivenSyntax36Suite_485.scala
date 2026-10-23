// Program 485 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 636, "Parameterized named
// typeclass with anonymous context parameter, braces"): scalac 3.8.4's parser accepts it, teq's reports "expected
// ':', found '['". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a
// node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 80
//     Defn.Object 0 79
//       Term.Name 7 11
//       Template 12 79
//         Template.Body 12 79
//           Defn.Given 14 76
//             Term.Name 20 23
//             Member.ParamClauseGroup 25 43
//               Type.ParamClause 25 28
//                 Type.Param 26 27
//                   Type.Name 26 27
//                   Type.ParamClause 27 27
//                   Type.Bounds 27 27
//               Term.ParamClause 32 40
//                 Term.Param 33 39
//                   Name.Anonymous 33 33
//                   Type.Apply 33 39
//                     Type.Name 33 36
//                     Type.ArgClause 36 39
//                       Type.Name 37 38
//             Template 44 76
//               Init 44 56
//                 Type.Apply 44 56
//                   Type.Name 44 47
//                   Type.ArgClause 47 56
//                     Type.Apply 48 55
//                       Type.Name 48 52
//                       Type.ArgClause 52 55
//                         Type.Name 53 54
//                 Name.Anonymous 57 57
//               Template.Body 57 76
//                 Defn.Def 61 74
//                   Term.Name 65 68
//                   Term.Name 71 74
object W485 {
given ord: [A] => (Ord[A]) => Ord[List[A]] {
  def foo = ???
}

}
