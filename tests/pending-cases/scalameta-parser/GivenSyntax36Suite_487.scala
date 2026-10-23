// Program 487 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 671, "context a lambda
// function, braces"): scalac 3.8.4's parser accepts it, teq's reports "expected an identifier, found '['". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 81
//     Defn.Object 0 80
//       Term.Name 7 11
//       Template 12 80
//         Template.Body 12 80
//           Defn.Given 14 77
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
//             Template 45 77
//               Init 45 57
//                 Type.Apply 45 57
//                   Type.Name 45 48
//                   Type.ArgClause 48 57
//                     Type.Apply 49 56
//                       Type.Name 49 53
//                       Type.ArgClause 53 56
//                         Type.Name 54 55
//                 Name.Anonymous 58 58
//               Template.Body 58 77
//                 Defn.Def 62 75
//                   Term.Name 66 69
//                   Term.Name 72 75
object W487 {
given ord: ([A] =>> Ord[A]) => Ord[List[A]] {
  def foo = ???
}

}
