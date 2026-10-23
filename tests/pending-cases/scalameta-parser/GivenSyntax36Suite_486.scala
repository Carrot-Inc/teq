// Program 486 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 654, "context a lambda
// function, coloneol"): scalac 3.8.4's parser accepts it, teq's reports "expected an identifier, found '['". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 78
//     Defn.Object 0 77
//       Term.Name 7 11
//       Template 12 77
//         Template.Body 12 77
//           Defn.Given 14 74
//             Term.Name 20 23
//             Member.ParamClauseGroup 25 44
//               Type.ParamClause 25 25
//               Term.ParamClause 25 41
//                 Term.Param 26 40
//                   Name.Anonymous 26 26
//                   Type.Lambda 26 40
//                     Type.ParamClause 26 29
//                       Type.Param 27 28
//                         Type.Name 27 28
//                         Type.ParamClause 28 28
//                         Type.Bounds 28 28
//                     Type.Apply 34 40
//                       Type.Name 34 37
//                       Type.ArgClause 37 40
//                         Type.Name 38 39
//             Template 45 74
//               Init 45 57
//                 Type.Apply 45 57
//                   Type.Name 45 48
//                   Type.ArgClause 48 57
//                     Type.Apply 49 56
//                       Type.Name 49 53
//                       Type.ArgClause 53 56
//                         Type.Name 54 55
//                 Name.Anonymous 57 57
//               Template.Body 57 74
//                 Defn.Def 61 74
//                   Term.Name 65 68
//                   Term.Name 71 74
object W486 {
given ord: ([A] =>> Ord[A]) => Ord[List[A]]:
  def foo = ???

}
