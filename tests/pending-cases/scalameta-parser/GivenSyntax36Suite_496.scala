// Program 496 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 835, "named, func arg types,
// coloneol"): scalac 3.8.4's parser accepts it, teq's reports "expected ':', found ')'". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 75
//     Defn.Object 0 74
//       Term.Name 7 11
//       Template 12 74
//         Template.Body 12 74
//           Defn.Given 14 71
//             Term.Name 20 23
//             Member.ParamClauseGroup 25 41
//               Type.ParamClause 25 25
//               Term.ParamClause 25 28
//                 Term.Param 26 27
//                   Name.Anonymous 26 26
//                   Type.Name 26 27
//               Term.ParamClause 32 38
//                 Term.Param 32 38
//                   Name.Anonymous 32 32
//                   Type.Apply 32 38
//                     Type.Name 32 35
//                     Type.ArgClause 35 38
//                       Type.Name 36 37
//             Template 42 71
//               Init 42 54
//                 Type.Apply 42 54
//                   Type.Name 42 45
//                   Type.ArgClause 45 54
//                     Type.Apply 46 53
//                       Type.Name 46 50
//                       Type.ArgClause 50 53
//                         Type.Name 51 52
//                 Name.Anonymous 54 54
//               Template.Body 54 71
//                 Defn.Def 58 71
//                   Term.Name 62 65
//                   Term.Name 68 71
object W496 {
given ord: (A) => Ord[A] => Ord[List[A]]:
  def foo = ???

}
