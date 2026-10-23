// Program 482 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 586, "Parameterized anonymous
// typeclass with anonymous context parameter, coloneol"): scalac 3.8.4's parser accepts it, teq's reports "expected
// ':', found '['". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a
// node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 72
//     Defn.Object 0 71
//       Term.Name 7 11
//       Template 12 71
//         Template.Body 12 71
//           Defn.Given 14 68
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
//             Template 39 68
//               Init 39 51
//                 Type.Apply 39 51
//                   Type.Name 39 42
//                   Type.ArgClause 42 51
//                     Type.Apply 43 50
//                       Type.Name 43 47
//                       Type.ArgClause 47 50
//                         Type.Name 48 49
//                 Name.Anonymous 51 51
//               Template.Body 51 68
//                 Defn.Def 55 68
//                   Term.Name 59 62
//                   Term.Name 65 68
object W482 {
given [A] => (Ord[A]) => Ord[List[A]]:
  def foo = ???

}
