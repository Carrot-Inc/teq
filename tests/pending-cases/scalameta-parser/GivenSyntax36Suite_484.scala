// Program 484 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 619, "Parameterized named
// typeclass with anonymous context parameter, coloneol"): scalac 3.8.4's parser accepts it, teq's reports "expected
// ':', found '['". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a
// node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 77
//     Defn.Object 0 76
//       Term.Name 7 11
//       Template 12 76
//         Template.Body 12 76
//           Defn.Given 14 73
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
//             Template 44 73
//               Init 44 56
//                 Type.Apply 44 56
//                   Type.Name 44 47
//                   Type.ArgClause 47 56
//                     Type.Apply 48 55
//                       Type.Name 48 52
//                       Type.ArgClause 52 55
//                         Type.Name 53 54
//                 Name.Anonymous 56 56
//               Template.Body 56 73
//                 Defn.Def 60 73
//                   Term.Name 64 67
//                   Term.Name 70 73
object W484 {
given ord: [A] => (Ord[A]) => Ord[List[A]]:
  def foo = ???

}
