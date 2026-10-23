// Program 483 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 603, "Parameterized anonymous
// typeclass with anonymous context parameter, braces"): scalac 3.8.4's parser accepts it, teq's reports "expected
// ':', found '['". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a
// node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 75
//     Defn.Object 0 74
//       Term.Name 7 11
//       Template 12 74
//         Template.Body 12 74
//           Defn.Given 14 71
//             Name.Anonymous 20 20
//             Member.ParamClauseGroup 20 38
//               Type.ParamClause 20 23
//                 Type.Param 21 22
//                   Type.Name 21 22
//                   Type.ParamClause 22 22
//                   Type.Bounds 22 22
//               Term.ParamClause 27 35
//                 Term.Param 28 34
//                   Name.Anonymous 28 28
//                   Type.Apply 28 34
//                     Type.Name 28 31
//                     Type.ArgClause 31 34
//                       Type.Name 32 33
//             Template 39 71
//               Init 39 51
//                 Type.Apply 39 51
//                   Type.Name 39 42
//                   Type.ArgClause 42 51
//                     Type.Apply 43 50
//                       Type.Name 43 47
//                       Type.ArgClause 47 50
//                         Type.Name 48 49
//                 Name.Anonymous 52 52
//               Template.Body 52 71
//                 Defn.Def 56 69
//                   Term.Name 60 63
//                   Term.Name 66 69
object W483 {
given [A] => (Ord[A]) => Ord[List[A]] {
  def foo = ???
}

}
